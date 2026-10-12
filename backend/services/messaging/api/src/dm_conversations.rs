use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use api_common::{dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClearDmRequest {
    #[serde(default)]
    for_everyone: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearDmResponse {
    cleared_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
struct HiddenDmsResponse {
    peers: Vec<Uuid>,
}

#[post("/me/dms/{peer_id}/clear")]
async fn clear_dm(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<ClearDmRequest>,
) -> Result<impl Responder, HttpError> {
    let cleared_at = app_state
        .dm_conversations_usecase
        .clear(UserId::new(auth_token.id), path.into_inner(), body.for_everyone)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ClearDmResponse { cleared_at })))
}

#[get("/me/hidden-dms")]
async fn list_hidden_dms(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let peers = app_state
        .dm_conversations_usecase
        .list_hidden(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(HiddenDmsResponse { peers })))
}

#[put("/me/hidden-dms/{peer_id}")]
async fn hide_dm(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    app_state
        .dm_conversations_usecase
        .set_hidden(UserId::new(auth_token.id), path.into_inner(), true)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/me/hidden-dms/{peer_id}")]
async fn unhide_dm(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    app_state
        .dm_conversations_usecase
        .set_hidden(UserId::new(auth_token.id), path.into_inner(), false)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn dm_conversation_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(clear_dm)
        .service(list_hidden_dms)
        .service(hide_dm)
        .service(unhide_dm);
}
