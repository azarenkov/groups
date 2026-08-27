use actix_web::{HttpResponse, Responder, get, post, web};
use contracts::internal::{
    attachments::{LinkedRequestV1, LinkedResponseV1},
    friendships::FriendsResponseV1,
    servers::{CanManageResponseV1, OwnerResponseV1},
    users::StateResponseV1,
};
use messaging_domain::{friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::repository::MessageRepository, server::id::ServerId, server::repository::ServerRepository};
use domain::user::id::UserId;
use uuid::Uuid;

use crate::state::AppState;
use crate::http_errors::HttpError;

#[get("/servers/{server_id}/can-manage/{user_id}")]
async fn server_can_manage(
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (server_id, user_id) = path.into_inner();
    let allowed = app_state
        .can_manage_server_usecase
        .execute(ServerId::new(server_id), UserId::new(user_id))
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(CanManageResponseV1 { allowed }))
}

#[get("/servers/{server_id}/owner")]
async fn server_owner(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let server_id = ServerId::new(path.into_inner());
    match app_state
        .server_repository
        .find_server_owner(&server_id)
        .await
        .map_err(HttpError::from)?
    {
        Some(owner) => Ok(HttpResponse::Ok().json(OwnerResponseV1 {
            owner_id: owner.uuid(),
        })),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

#[get("/users/{user_id}/has-state")]
async fn user_has_state(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let has_state = app_state
        .server_repository
        .has_user_messaging_state(path.into_inner())
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(StateResponseV1 { has_state }))
}

#[get("/friendships/{a}/{b}")]
async fn friendship_pair(
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (a, b) = path.into_inner();
    let pair = app_state
        .friendship_repository
        .find_pair(&UserId::new(a), &UserId::new(b))
        .await
        .map_err(HttpError::from)?;
    let friends = matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted);
    Ok(HttpResponse::Ok().json(FriendsResponseV1 { friends }))
}

#[post("/attachments/linked")]
async fn linked_attachments(
    app_state: web::Data<AppState>,
    body: web::Json<LinkedRequestV1>,
) -> Result<impl Responder, HttpError> {
    let linked_ids = app_state
        .message_repository
        .linked_attachment_subset(&body.attachment_ids)
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(LinkedResponseV1 { linked_ids }))
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/internal")
            .service(server_can_manage)
            .service(server_owner)
            .service(user_has_state)
            .service(friendship_pair)
            .service(linked_attachments),
    );
}
