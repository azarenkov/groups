use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, put, web};
use application::dto::requests::server::delete_server::DeleteServerRequest;
use domain::{server::id::ServerId, shared::pagination::Pagination, user::id::UserId};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::shared::{
    context::request_context,
    dto::{
        pagination::{PaginationMeta, PaginationQuery},
        requests::server::create_server::CreateServerRequestBody,
        responses::{
            ApiResponse,
            server::{
                create_server::CreateServerResponse,
                list_sent_server_invites::ListSentServerInvitesResponse,
                list_server_channels::ListServerChannelsResponse,
                list_server_members::ListServerMembersResponse,
                list_servers_by_user::ListServersByUserResponse,
            },
        },
    },
    error::ApiError,
    state::AppState,
};

#[post("")]
async fn create_server(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<CreateServerRequestBody>,
) -> Result<impl Responder, ApiError> {
    let body: CreateServerRequestBody = body.into_inner();
    body.validate()?;

    let ctx = request_context(&req);
    let server = app_state
        .create_server_usecase
        .execute(body.into_request(ctx), UserId::new(auth_token.id))
        .await?;

    let response = CreateServerResponse::from(server);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[get("")]
async fn list_servers_by_user(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;

    let user_id = UserId::new(auth_token.id);
    let pagination = Pagination::new(query.page, query.per_page)?;

    let result = app_state
        .list_servers_by_user_usecase
        .execute(user_id, pagination)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListServersByUserResponse::new(result.items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}

#[get("/{server_id}/members")]
async fn list_server_members(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let requester_id = UserId::new(auth_token.id);
    let pagination = Pagination::new(query.page, query.per_page)?;

    let result = app_state
        .list_server_members_usecase
        .execute(server_id, requester_id, pagination)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListServerMembersResponse::new(result.items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}

#[get("/{server_id}/channels")]
async fn list_server_channels(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let requester_id = UserId::new(auth_token.id);
    let pagination = Pagination::new(query.page, query.per_page)?;

    let result = app_state
        .list_server_channels_usecase
        .execute(server_id, requester_id, pagination)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListServerChannelsResponse::new(result.items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}

#[get("/{server_id}/voice-presence")]
async fn list_voice_presence(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let requester_id = UserId::new(auth_token.id);

    let snapshot = app_state
        .get_voice_presence_usecase
        .execute(server_id, requester_id)
        .await?;

    let items: Vec<VoicePresenceItem> = snapshot
        .presence
        .into_iter()
        .map(|r| VoicePresenceItem {
            channel_id: r.channel_id.uuid(),
            user_id: r.user_id.uuid(),
            user_name: r.user_name,
            is_muted: r.is_muted,
            camera_on: r.camera_on,
            screen_on: r.screen_on,
        })
        .collect();

    let stream_viewers: Vec<StreamViewerItem> = snapshot
        .stream_viewers
        .into_iter()
        .map(|r| StreamViewerItem {
            channel_id: r.channel_id.uuid(),
            streamer_id: r.streamer_id.uuid(),
            count: r.count,
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(VoicePresenceResponse {
        items,
        stream_viewers,
    })))
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VoicePresenceItem {
    channel_id: Uuid,
    user_id: Uuid,
    user_name: String,
    is_muted: bool,
    camera_on: bool,
    screen_on: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamViewerItem {
    channel_id: Uuid,
    streamer_id: Uuid,
    count: i64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VoicePresenceResponse {
    items: Vec<VoicePresenceItem>,
    stream_viewers: Vec<StreamViewerItem>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateStreamViewerBody {
    channel_id: Uuid,
    streamer_id: Uuid,
    watching: bool,
}

#[put("/{server_id}/stream-viewers")]
async fn update_stream_viewer(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateStreamViewerBody>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let viewer_id = UserId::new(auth_token.id);
    let body = body.into_inner();
    let ctx = request_context(&req);

    app_state
        .update_stream_viewer_usecase
        .execute(
            server_id,
            viewer_id,
            application::dto::requests::channel::update_stream_viewer::UpdateStreamViewerRequest {
                channel_id: body.channel_id,
                streamer_id: body.streamer_id,
                watching: body.watching,
                ctx,
            },
        )
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[get("/{server_id}/sent-invites")]
async fn list_sent_server_invites(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let sender_id = UserId::new(auth_token.id);

    let recipient_ids = app_state
        .list_sent_server_invites_usecase
        .execute(server_id, sender_id)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        ListSentServerInvitesResponse::new(recipient_ids),
    )))
}

#[delete("/{server_id}/members/me")]
async fn leave_server(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);

    app_state.leave_server_usecase.execute(server_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{server_id}")]
async fn delete_server(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let owner_id = UserId::new(auth_token.id);

    let data = DeleteServerRequest {
        server_id,
        owner_id,
        ctx: request_context(&req),
    };

    let deleted = app_state.delete_server_usecase.execute(data).await?;

    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
