use actix_web::{HttpResponse, Responder, get, web};
use domain::server::{id::ServerId, repository::ServerRepository};
use serde::Serialize;
use uuid::Uuid;

use crate::messaging::state::AppState;
use crate::shared::error::ApiError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnerResponse {
    owner_id: Uuid,
}

#[get("/servers/{server_id}/owner")]
async fn server_owner(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    match app_state
        .server_repository
        .find_server_owner(&server_id)
        .await
        .map_err(ApiError::from)?
    {
        Some(owner) => Ok(HttpResponse::Ok().json(OwnerResponse {
            owner_id: owner.uuid(),
        })),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/internal").service(server_owner));
}
