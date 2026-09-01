use std::collections::HashMap;

use actix_web::{HttpResponse, Responder, delete, get, post, web};
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use identity_domain::user::id::UserId;
use serde::Serialize;
use uuid::Uuid;

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionItem {
    provider: String,
    display_name: String,
    url: Option<String>,
}

fn connection_url(provider: &str, subject: &str, display_name: &str) -> Option<String> {
    match provider {
        "github" if !display_name.is_empty() => {
            Some(format!("https://github.com/{display_name}"))
        }
        "steam" => Some(format!("https://steamcommunity.com/profiles/{subject}")),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
struct ConnectionsResponse {
    connections: Vec<ConnectionItem>,
}

#[derive(Debug, Serialize)]
struct StartConnectionResponse {
    url: String,
}

#[get("/me/connections")]
pub async fn list_connections(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let items = app_state
        .list_connections_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ConnectionsResponse {
        connections: items
            .into_iter()
            .map(|c| ConnectionItem {
                url: connection_url(&c.provider, &c.subject, &c.display_name),
                provider: c.provider,
                display_name: c.display_name,
            })
            .collect(),
    })))
}

#[get("/{user_id}/connections")]
pub async fn user_connections(
    _auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let items = app_state
        .list_connections_usecase
        .execute(UserId::new(path.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ConnectionsResponse {
        connections: items
            .into_iter()
            .map(|c| ConnectionItem {
                url: connection_url(&c.provider, &c.subject, &c.display_name),
                provider: c.provider,
                display_name: c.display_name,
            })
            .collect(),
    })))
}

#[post("/me/connections/{provider}/start")]
pub async fn start_connection(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, HttpError> {
    let url = app_state
        .start_connection_usecase
        .execute(UserId::new(auth_token.id), &path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(StartConnectionResponse { url })))
}

#[delete("/me/connections/{provider}")]
pub async fn remove_connection(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, HttpError> {
    app_state
        .remove_connection_usecase
        .execute(UserId::new(auth_token.id), &path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{provider}/callback")]
pub async fn connection_callback(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let provider = path.into_inner();
    let flag = match app_state
        .complete_connection_usecase
        .execute(&provider, &query)
        .await
    {
        Ok(()) => "ok",
        Err(e) => {
            tracing::warn!(error = %e, provider = %provider, "connection callback failed");
            "error"
        }
    };
    HttpResponse::Found()
        .append_header((
            "Location",
            format!("{}/?connections={}", app_state.public_app_url, flag),
        ))
        .finish()
}
