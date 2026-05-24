use std::time::{Duration, Instant};

use actix_web::{HttpRequest, HttpResponse, get, web};
use actix_ws::AggregatedMessage;
use futures::StreamExt;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;
use shared::middleware::jwt::claims::{AccessTokenClaims, TokenType};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::connections::{Connection, ConnectionManager};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
struct WsQuery {
    #[serde(rename = "token", alias = "access_token")]
    token: Option<String>,
}

#[derive(Clone)]
pub struct WsState {
    pub manager: ConnectionManager,
    pub jwt_secret: String,
    pub outbound_buffer: usize,
}

#[get("/ws")]
pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<WsQuery>,
    state: web::Data<WsState>,
) -> Result<HttpResponse, actix_web::Error> {
    let token = match query.token.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(HttpResponse::Unauthorized().body("missing token")),
    };

    let user_id = match decode::<AccessTokenClaims>(
        token,
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
            decoded.claims.sub
        }
        Err(_) => return Ok(HttpResponse::Unauthorized().body("invalid token")),
    };

    let (response, session, msg_stream) = actix_ws::handle(&req, body)?;
    let manager = state.manager.clone();
    let outbound_buffer = state.outbound_buffer;

    actix_web::rt::spawn(connection_task(
        manager,
        user_id,
        session,
        msg_stream,
        outbound_buffer,
    ));

    Ok(response)
}

async fn connection_task(
    manager: ConnectionManager,
    user_id: Uuid,
    mut session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
    outbound_buffer: usize,
) {
    let (tx, mut rx) = mpsc::channel::<String>(outbound_buffer);
    let conn = Connection::new(user_id, tx);
    let conn_id = conn.id;
    manager.register(conn);

    tracing::info!(%user_id, %conn_id, "WS connection opened");

    let mut aggregated = msg_stream
        .max_frame_size(MAX_FRAME_BYTES)
        .aggregate_continuations()
        .max_continuation_size(MAX_FRAME_BYTES);

    let mut hb_interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    hb_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_seen = Instant::now();

    let close_reason = loop {
        tokio::select! {
            biased;

            tick = hb_interval.tick() => {
                if tick.into_std().duration_since(last_seen) > CLIENT_TIMEOUT {
                    tracing::info!(%user_id, %conn_id, "WS client timed out");
                    break Some("client timeout");
                }
                if session.ping(b"").await.is_err() {
                    break Some("ping failed");
                }
            }

            Some(out) = rx.recv() => {
                if session.text(out).await.is_err() {
                    break Some("send failed");
                }
            }

            incoming = aggregated.next() => {
                match incoming {
                    Some(Ok(AggregatedMessage::Ping(bytes))) => {
                        last_seen = Instant::now();
                        if session.pong(&bytes).await.is_err() {
                            break Some("pong failed");
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
                        break Some("protocol error");
                    }
                    None => break None,
                }
            }
        }
    };

    manager.unregister(user_id, conn_id);
    if let Some(r) = close_reason {
        let _ = session
            .close(Some(actix_ws::CloseReason {
                code: actix_ws::CloseCode::Away,
                description: Some(r.to_string()),
            }))
            .await;
    } else {
        let _ = session.close(None).await;
    }
    tracing::info!(%user_id, %conn_id, "WS connection closed");
}

