use actix_web::{HttpResponse, Responder, post, web};
use contracts::internal::users::{ResolveUsersRequestV1, ResolvedUserV1};
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

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(resolve_users);
}
