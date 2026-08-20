use actix_web::{HttpResponse, Responder, get, post, web};
use domain::server::{id::ServerId, repository::ServerRepository};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StateResponse {
    has_state: bool,
}

#[get("/users/{user_id}/has-state")]
async fn user_has_state(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let has_state = app_state
        .server_repository
        .has_user_messaging_state(path.into_inner())
        .await
        .map_err(ApiError::from)?;
    Ok(HttpResponse::Ok().json(StateResponse { has_state }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkedQuery {
    attachment_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LinkedResponse {
    linked_ids: Vec<Uuid>,
}

#[post("/attachments/linked")]
async fn linked_attachments(
    app_state: web::Data<AppState>,
    body: web::Json<LinkedQuery>,
) -> Result<impl Responder, ApiError> {
    let linked_ids = app_state
        .message_repository
        .linked_attachment_subset(&body.attachment_ids)
        .await
        .map_err(ApiError::from)?;
    Ok(HttpResponse::Ok().json(LinkedResponse { linked_ids }))
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/internal")
            .service(server_owner)
            .service(user_has_state)
            .service(linked_attachments),
    );
}
