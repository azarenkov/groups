use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, web};
use application::dto::requests::channel::{
    create_channel::CreateChannelRequest, delete_channel::DeleteChannelRequest,
    join_voice_channel::JoinVoiceChannelRequest,
    list_channel_participants::ListChannelParticipantsRequest, mute_channel::MuteChannelRequest,
    unmute_channel::UnmuteChannelRequest,
};
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
            create_channel::CreateChannelRequestBody,
            join_voice_channel::JoinVoiceChannelRequestBody, mute_channel::MuteRequestBody,
            unmute_channel::UnmuteRequestBody,
        },
        responses::{
            ApiResponse,
            channel::{
                create_channel::CreateChannelResponse,
                join_voice_channel::JoinVoiceChannelResponse,
                list_channel_participants::ListChannelParticipantsResponse,
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
