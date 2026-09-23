use actix_web::{HttpResponse, Responder, post, web};
use contracts::internal::users::{ResolveUsersRequestV1, ResolvedUserV1};
use identity::ports::activity_store::ActivityStore;
use identity_domain::user::repository::UserRepository;

use crate::state::AppState;
use crate::http_errors::HttpError;

#[post("/internal/users/resolve")]
async fn resolve_users(
    app_state: web::Data<AppState>,
    body: web::Json<ResolveUsersRequestV1>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    if body.names.is_empty() || body.names.len() > 500 {
        return Err(HttpError::bad_request("Invalid names".to_string()));
    }
    let names: Vec<String> = body
        .names
        .into_iter()
        .map(|n| n.to_lowercase())
        .collect();
    let users = app_state
        .user_repository
        .ids_by_names(&names)
        .await
        .map_err(HttpError::from_error)?;
    let items: Vec<ResolvedUserV1> = users
        .into_iter()
        .map(|(id, name)| ResolvedUserV1 {
            id: id.uuid(),
            name: name.as_str().to_string(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(items))
}


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
    cfg.service(resolve_users).service(set_activity_visibility);
}
