use actix_web::{HttpResponse, Responder, post, web};
use identity::ports::activity_store::ActivityStore;

use crate::state::AppState;
use crate::http_errors::HttpError;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityVisibilityBody {
    user_id: uuid::Uuid,
    hidden: bool,
}

#[post("/internal/users/activity-visibility")]
async fn set_activity_visibility(
    app_state: web::Data<AppState>,
    body: web::Json<ActivityVisibilityBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    app_state
        .activity_store
        .set_hidden(&domain::user::id::UserId::new(body.user_id), body.hidden)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, user_id = %body.user_id, "Failed to store activity visibility");
            HttpError::new(
                "Failed to store activity visibility".to_string(),
                actix_web::http::StatusCode::BAD_GATEWAY,
                "ACTIVITY_VISIBILITY_FAILED",
            )
        })?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(set_activity_visibility);
}
