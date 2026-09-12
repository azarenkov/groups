use actix_web::{HttpResponse, Responder, get, post, web};
use contracts::internal::{
    attachments::{LinkedRequestV1, LinkedResponseV1},
    calls::CallPermitResponseV1,
    friendships::{BlockRelationResponseV1, BlockRelationsRequestV1, BlockRelationsResponseV1, FriendsResponseV1},
    servers::{CanManageResponseV1, OwnerResponseV1},
    users::{ActivityAudienceRequestV1, ActivityAudienceResponseV1, DmPeersRequestV1, DmPeersResponseV1, StateResponseV1},
};
use messaging_domain::{dm_message::repository::DmMessageRepository, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::repository::MessageRepository, privacy::ContactPolicy, privacy::PrivacyRepository, server::id::ServerId, server::repository::ServerRepository};
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

#[get("/blocks/{a}/{b}")]
async fn block_pair(
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (a, b) = path.into_inner();
    let relation = app_state
        .friendship_repository
        .block_relation(&UserId::new(a), &UserId::new(b))
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(BlockRelationResponseV1 {
        blocking: relation.blocking,
        blocked_by: relation.blocked_by,
    }))
}

#[post("/users/{user_id}/block-relations")]
async fn block_relations(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<BlockRelationsRequestV1>,
) -> Result<impl Responder, HttpError> {
    let user = UserId::new(path.into_inner());
    let candidates: Vec<UserId> = body.ids.iter().copied().map(UserId::new).collect();
    let blocking = app_state
        .friendship_repository
        .blocked_among(&user, &candidates)
        .await
        .map_err(HttpError::from)?;
    let blocked_by = app_state
        .friendship_repository
        .blockers_among(&user, &candidates)
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(BlockRelationsResponseV1 {
        blocking: blocking.into_iter().map(|u| u.uuid()).collect(),
        blocked_by: blocked_by.into_iter().map(|u| u.uuid()).collect(),
    }))
}

#[post("/users/{user_id}/dm-peers")]
async fn dm_peers(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<DmPeersRequestV1>,
) -> Result<impl Responder, HttpError> {
    let user = UserId::new(path.into_inner());
    let candidates: Vec<UserId> = body.ids.iter().copied().map(UserId::new).collect();
    let peers = app_state
        .dm_repository
        .peers_with_history(&user, &candidates)
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(DmPeersResponseV1 {
        ids: peers.into_iter().map(|u| u.uuid()).collect(),
    }))
}

#[get("/calls/permit/{caller}/{callee}")]
async fn call_permit(
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (caller, callee) = path.into_inner();
    let caller = UserId::new(caller);
    let callee = UserId::new(callee);
    let allowed = call_allowed(&app_state, &caller, &callee).await?;
    Ok(HttpResponse::Ok().json(CallPermitResponseV1 { allowed }))
}

pub(crate) async fn call_allowed(
    app_state: &AppState,
    caller: &UserId,
    callee: &UserId,
) -> Result<bool, HttpError> {
    if app_state
        .friendship_repository
        .block_relation(caller, callee)
        .await
        .map_err(HttpError::from)?
        .any()
    {
        return Ok(false);
    }
    let settings = app_state
        .privacy_repository
        .get(callee)
        .await
        .map_err(HttpError::from)?;
    Ok(match settings.call_policy {
        ContactPolicy::All => true,
        ContactPolicy::NoOne => false,
        ContactPolicy::Friends => {
            let pair = app_state
                .friendship_repository
                .find_pair(caller, callee)
                .await
                .map_err(HttpError::from)?;
            matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted)
        }
    })
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

#[post("/users/{user_id}/activity-audience")]
async fn activity_audience(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<ActivityAudienceRequestV1>,
) -> Result<impl Responder, HttpError> {
    let ids = app_state
        .server_repository
        .activity_audience(path.into_inner(), &body.ids)
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(ActivityAudienceResponseV1 { ids }))
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/internal")
            .service(activity_audience)
            .service(server_can_manage)
            .service(server_owner)
            .service(user_has_state)
            .service(friendship_pair)
            .service(block_pair)
            .service(block_relations)
            .service(dm_peers)
            .service(call_permit)
            .service(linked_attachments),
    );
}
