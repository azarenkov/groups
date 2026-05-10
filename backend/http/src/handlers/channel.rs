use actix_web::{HttpResponse, Responder, delete, post, web};
use application::dto::requests::channel::{
    join_channel::JoinChannelRequest as AppJoinChannelRequest,
    leave_channel::LeaveChannelRequest as AppLeftChannelRequest,
};
use domain::{channel::id::ChannelId, user::id::UserId};
use uuid::Uuid;
use validator::Validate;

use crate::{
    middleware::auth::AuthToken,
    shared::{
        dto::{
            requests::channel::{
                create_channel::CreateChannelRequest, join_channel::JoinChannelRequest,
                leave_channel::LeaveChannelRequest,
            },
            responses::{
                ApiResponse,
                channel::{
                    create_channel::CreateChannelResponse, join_channel::JoinChannelResponse,
                },
            },
        },
        error::ApiError,
        state::AppState,
    },
};

#[post("/create")]
async fn create_channel(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<CreateChannelRequest>,
) -> Result<impl Responder, ApiError> {
    let request: CreateChannelRequest = request.into_inner();
    request.validate()?;

    let user_id = UserId::new(auth_token.id);

    let channel = app_state
        .create_channel_usecase
        .execute(request.into(), user_id)
        .await?;

    let response = CreateChannelResponse::from(channel);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[post("/{channel_id}/join")]
async fn join_channel(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    request: web::Json<JoinChannelRequest>,
) -> Result<impl Responder, ApiError> {
    let channel_id = path.into_inner();
    let request = request.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = AppJoinChannelRequest::new(channel_id, request.server_id, request.password);

    let response: JoinChannelResponse = app_state
        .join_channel_usecase
        .execute(data, user_id)
        .await?
        .into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/{channel_id}/leave")]
async fn leave_channel(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    request: web::Json<LeaveChannelRequest>,
) -> Result<impl Responder, ApiError> {
    let channel_id = path.into_inner();
    let request = request.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = AppLeftChannelRequest::new(channel_id, request.server_id);

    app_state
        .leave_channel_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{channel_id}")]
async fn delete_channel(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let channel_id = ChannelId::new(path.into_inner());
    let owner_id = UserId::new(auth_token.id);

    app_state
        .delete_channel_usecase
        .execute(channel_id, owner_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
