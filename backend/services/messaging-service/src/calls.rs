use actix_web::{HttpResponse, Responder, post, web};
use domain::user::id::UserId;
use serde::Deserialize;
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::state::AppState;
use api_common::error::ApiError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogCallBody {
    recipient_id: Uuid,
    status: String,
    duration_seconds: Option<i32>,
}

#[post("/log")]
async fn log_call(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<LogCallBody>,
) -> Result<impl Responder, ApiError> {
    let sender_id = UserId::new(auth_token.id);
    let body = body.into_inner();
    app_state
        .log_call_usecase
        .execute(sender_id, body.recipient_id, body.status, body.duration_seconds)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn call_log_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/v1/calls").service(log_call));
}
