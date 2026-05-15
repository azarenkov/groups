use actix_web::{HttpRequest, HttpResponse, Responder, http, post, web};

use crate::shared::{error::ApiError, state::AppState};

#[post("/livekit")]
async fn livekit_webhook(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Bytes,
) -> Result<impl Responder, ApiError> {
    let auth_token = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError::new(
                "Missing authorization header".to_string(),
                http::StatusCode::UNAUTHORIZED,
                "WEBHOOK_MISSING_AUTH",
            )
        })?;

    let body_str = std::str::from_utf8(&body).map_err(|_| {
        ApiError::new(
            "Body is not valid UTF-8".to_string(),
            http::StatusCode::BAD_REQUEST,
            "WEBHOOK_INVALID_BODY",
        )
    })?;

    app_state
        .handle_livekit_webhook_usecase
        .execute(body_str, auth_token)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
