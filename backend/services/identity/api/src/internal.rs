use actix_web::{HttpResponse, Responder, post, web};
use domain::user::id::UserId;

use crate::http_errors::HttpError;
use crate::state::AppState;

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
    app_state
        .set_presence_preference_usecase
        .execute(UserId::new(body.user_id), body.hidden, None)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(set_activity_visibility);
}
