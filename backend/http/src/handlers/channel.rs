use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, web};
use application::dto::requests::channel::{
    create_channel::CreateChannelRequest, delete_channel::DeleteChannelRequest,
    delete_channel_message::DeleteChannelMessageRequest,
    edit_channel_message::EditChannelMessageRequest,
    join_text_channel::JoinTextChannelRequest, join_voice_channel::JoinVoiceChannelRequest,
    list_channel_messages::ListChannelMessagesRequest,
    list_channel_participants::ListChannelParticipantsRequest, mute_channel::MuteChannelRequest,
    send_channel_message::SendChannelMessageRequest, unmute_channel::UnmuteChannelRequest,
};
use serde::Deserialize;
use domain::{
    channel::id::ChannelId, server::id::ServerId, shared::pagination::Pagination,
    user::id::UserId,
};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::shared::{
    context::request_context,
    dto::{
        pagination::{PaginationMeta, PaginationQuery},
        requests::channel::{
            create_channel::CreateChannelRequestBody, edit_message::EditMessageRequestBody,
            join_voice_channel::JoinVoiceChannelRequestBody, mute_channel::MuteRequestBody,
            send_message::SendMessageRequestBody, unmute_channel::UnmuteRequestBody,
        },
        responses::{
            ApiResponse,
            channel::{
                create_channel::CreateChannelResponse,
                join_voice_channel::JoinVoiceChannelResponse,
                list_channel_participants::ListChannelParticipantsResponse,
                message::{ListChannelMessagesResponse, MessageResponse},
            },
        },
    },
    error::ApiError,
    state::AppState,
};

#[post("/{server_id}/channels")]
async fn create_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<CreateChannelRequestBody>,
) -> Result<impl Responder, ApiError> {
    let server_id = path.into_inner();
    let body: CreateChannelRequestBody = body.into_inner();
    body.validate()?;

    let user_id = UserId::new(auth_token.id);

    let data = CreateChannelRequest::new(
        server_id,
        body.name,
        body.kind.into(),
        body.visibility.into(),
        body.password,
        request_context(&req),
    );

    let channel = app_state
        .create_channel_usecase
        .execute(data, user_id)
        .await?;

    let response = CreateChannelResponse::from(channel);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[post("/{server_id}/channels/{channel_id}/join_voice")]
async fn join_voice_channel(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<JoinVoiceChannelRequestBody>,
) -> Result<impl Responder, ApiError> {
    let (server_id, channel_id) = path.into_inner();
    let body = body.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = JoinVoiceChannelRequest::new(channel_id, server_id, body.password);

    let response: JoinVoiceChannelResponse = app_state
        .join_voice_channel_usecase
        .execute(data, user_id)
        .await?
        .into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/{server_id}/channels/{channel_id}/mute")]
async fn mute_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<MuteRequestBody>,
) -> Result<HttpResponse, ApiError> {
    let (server_id, channel_id) = path.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = MuteChannelRequest {
        channel_id,
        server_id,
        target_user_id: body.target_user_id,
        ctx: request_context(&req),
    };

    app_state
        .mute_channel_usecase
        .execute(data, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{server_id}/channels/{channel_id}/unmute")]
async fn unmute_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UnmuteRequestBody>,
) -> Result<HttpResponse, ApiError> {
    let (server_id, channel_id) = path.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = UnmuteChannelRequest {
        channel_id,
        server_id,
        target_user_id: body.target_user_id,
        ctx: request_context(&req),
    };

    app_state
        .unmute_channel_usecase
        .execute(data, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{server_id}/channels/{channel_id}/participants")]
async fn list_channel_participants(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;

    let (server_id, channel_id) = path.into_inner();
    let pagination = Pagination::new(query.page, query.per_page)?;

    let data = ListChannelParticipantsRequest::new(
        server_id,
        channel_id,
        auth_token.id,
        pagination,
        request_context(&req),
    );

    let result = app_state
        .list_channel_participants_usecase
        .execute(data)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListChannelParticipantsResponse::new(result.items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}

#[post("/{server_id}/channels/{channel_id}/join")]
async fn join_text_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    let (server_id, channel_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = JoinTextChannelRequest::new(server_id, channel_id, request_context(&req));

    app_state
        .join_text_channel_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

#[get("/{server_id}/channels/{channel_id}/messages")]
async fn list_channel_messages(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<ListMessagesQuery>,
) -> Result<impl Responder, ApiError> {
    let (server_id, channel_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = ListChannelMessagesRequest::new(
        server_id,
        channel_id,
        query.before,
        query.after,
        query.limit.unwrap_or(0),
        request_context(&req),
    );

    let messages = app_state
        .list_channel_messages_usecase
        .execute(data, user_id)
        .await?;

    let response =
        ListChannelMessagesResponse::new(messages.into_iter().map(MessageResponse::from).collect());

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/{server_id}/channels/{channel_id}/messages")]
async fn send_channel_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<SendMessageRequestBody>,
) -> Result<impl Responder, ApiError> {
    let (server_id, channel_id) = path.into_inner();
    let body = body.into_inner();
    body.validate()?;

    let user_id = UserId::new(auth_token.id);

    let data = SendChannelMessageRequest::new(
        server_id,
        channel_id,
        body.content,
        request_context(&req),
    );

    let response = app_state
        .send_channel_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(MessageResponse::from(response))))
}

#[patch("/{server_id}/channels/{channel_id}/messages/{message_id}")]
async fn edit_channel_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
    body: web::Json<EditMessageRequestBody>,
) -> Result<HttpResponse, ApiError> {
    let (server_id, channel_id, message_id) = path.into_inner();
    let body = body.into_inner();
    body.validate()?;

    let user_id = UserId::new(auth_token.id);

    let data = EditChannelMessageRequest::new(
        server_id,
        channel_id,
        message_id,
        body.content,
        request_context(&req),
    );

    app_state
        .edit_channel_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{server_id}/channels/{channel_id}/messages/{message_id}")]
async fn delete_channel_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    let (server_id, channel_id, message_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = DeleteChannelMessageRequest::new(
        server_id,
        channel_id,
        message_id,
        request_context(&req),
    );

    app_state
        .delete_channel_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{server_id}/channels/{channel_id}")]
async fn delete_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, ApiError> {
    let (server_id, channel_id) = path.into_inner();
    let channel_id = ChannelId::new(channel_id);
    let server_id = ServerId::new(server_id);
    let owner_id = UserId::new(auth_token.id);

    let data = DeleteChannelRequest {
        server_id,
        channel_id,
        deleted_by: owner_id,
        ctx: request_context(&req),
    };

    app_state.delete_channel_usecase.execute(data).await?;

    Ok(HttpResponse::NoContent().finish())
}
