use std::time::{Duration, Instant};

use actix_web::{HttpRequest, HttpResponse, get, http::header, post, web};
use actix_ws::AggregatedMessage;
use futures::StreamExt;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;
use api_common::middleware::jwt::claims::{AccessTokenClaims, TokenType};
use tokio::sync::mpsc;
use uuid::Uuid;

use realtime_core::usecases::block_aware_directory::BlockAwareDirectory;
use realtime_infrastructure::{connections::{Connection, ConnectionManager}, database::postgres::projection::PostgresRealtimeProjection, media_client::relations::MessagingRelationsClient};

pub type PresenceDirectory = BlockAwareDirectory<PostgresRealtimeProjection, MessagingRelationsClient>;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_FRAME_BYTES: usize = 64 * 1024;
const AUTH_PROTOCOL_PREFIX: &str = "bearer.";

#[derive(Clone)]
pub struct WsState {
    pub manager: ConnectionManager<PresenceDirectory>,
    pub jwt_secret: String,
    pub outbound_buffer: usize,
    pub access_revocations: api_common::middleware::revocation::AccessRevocations,
}

fn extract_bearer_token(req: &HttpRequest) -> Option<(String, String)> {
    let raw = req
        .headers()
        .get(header::SEC_WEBSOCKET_PROTOCOL)?
        .to_str()
        .ok()?;
    for value in raw.split(',') {
        let trimmed = value.trim();
        if let Some(token) = trimmed.strip_prefix(AUTH_PROTOCOL_PREFIX)
            && !token.is_empty()
        {
            return Some((trimmed.to_string(), token.to_string()));
        }
    }
    None
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresencePreferenceBody {
    pub appear_offline: bool,
}

fn extract_bearer_token_from_auth_header(req: &HttpRequest) -> Option<String> {
    let raw = req
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    raw.strip_prefix("Bearer ").map(|s| s.trim().to_string())
}

async fn authenticate_request(req: &HttpRequest, state: &WsState) -> Option<Uuid> {
    let secret = state.jwt_secret.as_str();
    let token = extract_bearer_token_from_auth_header(req)?;
    let decoded = decode::<AccessTokenClaims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .ok()?;
    if decoded.claims.token_type != TokenType::Access || !decoded.claims.verified {
        return None;
    }
    match state
        .access_revocations
        .is_revoked(&decoded.claims.sub, decoded.claims.iat)
        .await
    {
        Ok(false) => Some(decoded.claims.sub),
        Ok(true) => None,
        Err(e) => {
            tracing::warn!(error = %e, "Revocation check failed; rejecting request");
            None
        }
    }
}

#[derive(Deserialize)]
pub struct PresenceSnapshotQuery {
    pub ids: String,
}

#[derive(serde::Serialize)]
struct PresenceSnapshotResponse {
    online: Vec<Uuid>,
}

#[get("/presence")]
pub async fn presence_snapshot(
    req: HttpRequest,
    state: web::Data<WsState>,
    query: web::Query<PresenceSnapshotQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let Some(user_id) = authenticate_request(&req, &state).await else {
        return Ok(HttpResponse::Unauthorized().body("invalid token"));
    };
    let mut ids: Vec<Uuid> = Vec::new();
    for raw in query.ids.split(',') {
        if let Ok(id) = Uuid::parse_str(raw.trim())
            && !ids.contains(&id)
        {
            ids.push(id);
        }
        if ids.len() >= 100 {
            break;
        }
    }
    match state.manager.visible_online(user_id, &ids).await {
        Ok(online) => Ok(HttpResponse::Ok().json(PresenceSnapshotResponse { online })),
        Err(e) => {
            tracing::warn!(error = %e, %user_id, "Presence snapshot lookup failed");
            Ok(HttpResponse::ServiceUnavailable().body("presence lookup unavailable"))
        }
    }
}

#[post("/presence")]
pub async fn presence_preference(
    req: HttpRequest,
    state: web::Data<WsState>,
    body: web::Json<PresencePreferenceBody>,
) -> Result<HttpResponse, actix_web::Error> {
    let Some(user_id) = authenticate_request(&req, &state).await else {
        return Ok(HttpResponse::Unauthorized().body("invalid token"));
    };
    state
        .manager
        .set_appear_offline(user_id, body.appear_offline)
        .await;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/ws")]
pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<WsState>,
) -> Result<HttpResponse, actix_web::Error> {
    let Some((selected_protocol, token)) = extract_bearer_token(&req) else {
        return Ok(HttpResponse::Unauthorized().body("missing token"));
    };

    let (user_id, token_exp, token_iat) = match decode::<AccessTokenClaims>(
        &token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    ) {
        Ok(decoded) => {
            if decoded.claims.token_type != TokenType::Access {
                return Ok(HttpResponse::Unauthorized().body("invalid token type"));
            }
            if !decoded.claims.verified {
                return Ok(HttpResponse::Forbidden().body("email not verified"));
            }
            match state
                .access_revocations
                .is_revoked(&decoded.claims.sub, decoded.claims.iat)
                .await
            {
                Ok(false) => {}
                Ok(true) => return Ok(HttpResponse::Unauthorized().body("token revoked")),
                Err(e) => {
                    tracing::warn!(error = %e, "Revocation check failed; rejecting websocket");
                    return Ok(HttpResponse::ServiceUnavailable().body("revocation check unavailable"));
                }
            }
            (decoded.claims.sub, decoded.claims.exp, decoded.claims.iat)
        }
        Err(_) => return Ok(HttpResponse::Unauthorized().body("invalid token")),
    };

    let (mut response, session, msg_stream) = actix_ws::handle(&req, body)?;
    if let Ok(value) = header::HeaderValue::from_str(&selected_protocol) {
        response
            .headers_mut()
            .insert(header::SEC_WEBSOCKET_PROTOCOL, value);
    }
    let manager = state.manager.clone();
    let outbound_buffer = state.outbound_buffer;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let ttl_secs = (token_exp - now_secs).max(0) as u64;
    let token_ttl = Duration::from_secs(ttl_secs);

    actix_web::rt::spawn(connection_task(
        manager,
        user_id,
        session,
        msg_stream,
        outbound_buffer,
        ConnectionAuth {
            token_ttl,
            token_iat,
            access_revocations: state.access_revocations.clone(),
        },
    ));

    Ok(response)
}

struct ConnectionAuth {
    token_ttl: Duration,
    token_iat: i64,
    access_revocations: api_common::middleware::revocation::AccessRevocations,
}

async fn connection_task(
    manager: ConnectionManager<PresenceDirectory>,
    user_id: Uuid,
    mut session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
    outbound_buffer: usize,
    auth: ConnectionAuth,
) {
    let ConnectionAuth {
        token_ttl,
        token_iat,
        access_revocations,
    } = auth;
    let (tx, mut rx) = mpsc::channel::<String>(outbound_buffer);
    let conn = Connection::new(user_id, tx);
    let conn_id = conn.id;
    if let Err(e) = manager.register(conn).await {
        tracing::error!(error = %e, %user_id, %conn_id, "Failed to register connection; closing socket");
        let _ = session
            .close(Some(actix_ws::CloseReason {
                code: actix_ws::CloseCode::Error,
                description: Some("realtime backend unavailable".to_string()),
            }))
            .await;
        return;
    }

    tracing::info!(%user_id, %conn_id, "WS connection opened");

    let mut aggregated = msg_stream
        .max_frame_size(MAX_FRAME_BYTES)
        .aggregate_continuations()
        .max_continuation_size(MAX_FRAME_BYTES);

    let mut hb_interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    hb_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_seen = Instant::now();
    let token_expiry = tokio::time::sleep(token_ttl);
    tokio::pin!(token_expiry);

    let close_reason: Option<(actix_ws::CloseCode, &'static str)> = loop {
        tokio::select! {
            _ = &mut token_expiry => {
                tracing::info!(%user_id, %conn_id, "WS access token expired; closing");
                break Some((actix_ws::CloseCode::Other(4001), "token expired"));
            }

            _ = hb_interval.tick() => {
                match access_revocations.is_revoked(&user_id, token_iat).await {
                    Ok(false) => {}
                    Ok(true) => {
                        tracing::info!(%user_id, %conn_id, "WS access token revoked; closing");
                        break Some((actix_ws::CloseCode::Other(4001), "token revoked"));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, %user_id, %conn_id, "WS revocation check failed; closing");
                        break Some((actix_ws::CloseCode::Error, "revocation check unavailable"));
                    }
                }
                if last_seen.elapsed() >= CLIENT_TIMEOUT {
                    tracing::info!(%user_id, %conn_id, "WS client timed out");
                    break Some((actix_ws::CloseCode::Away, "client timeout"));
                }
                if session.ping(b"").await.is_err() {
                    break Some((actix_ws::CloseCode::Away, "ping failed"));
                }
            }

            out = rx.recv() => {
                let Some(payload) = out else {
                    break Some((actix_ws::CloseCode::Away, "backend channel closed"));
                };
                if session.text(payload).await.is_err() {
                    break Some((actix_ws::CloseCode::Away, "send failed"));
                }
            }

            incoming = aggregated.next() => {
                match incoming {
                    Some(Ok(AggregatedMessage::Ping(bytes))) => {
                        last_seen = Instant::now();
                        if session.pong(&bytes).await.is_err() {
                            break Some((actix_ws::CloseCode::Away, "pong failed"));
                        }
                    }
                    Some(Ok(AggregatedMessage::Pong(_))) => {
                        last_seen = Instant::now();
                    }
                    Some(Ok(AggregatedMessage::Text(_))) | Some(Ok(AggregatedMessage::Binary(_))) => {
                        last_seen = Instant::now();
                    }
                    Some(Ok(AggregatedMessage::Close(reason))) => {
                        tracing::info!(%user_id, %conn_id, ?reason, "WS close frame");
                        break None;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, %user_id, %conn_id, "WS protocol error");
                        break Some((actix_ws::CloseCode::Away, "protocol error"));
                    }
                    None => break None,
                }
            }
        }
    };

    manager.unregister(user_id, conn_id).await;
    if let Some((code, description)) = close_reason {
        let _ = session
            .close(Some(actix_ws::CloseReason {
                code,
                description: Some(description.to_string()),
            }))
            .await;
    } else {
        let _ = session.close(None).await;
    }
    tracing::info!(%user_id, %conn_id, "WS connection closed");
}

