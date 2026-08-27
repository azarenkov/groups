use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, put, web};
use messaging::dto::requests::server::delete_server::DeleteServerRequest;
use domain::{server::id::ServerId, shared::pagination::Pagination, user::id::UserId};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;
use api_common::{context::request_context};
use api_common::dto::{pagination::{PaginationMeta, PaginationQuery}, responses::ApiResponse};
use crate::dto::{requests::server::{ create_server::CreateServerRequestBody, update_server::UpdateServerRequestBody, }, responses::server::{ create_server::CreateServerResponse, invite_codes::{InviteCodeResponse, ListInviteCodesResponse}, list_sent_server_invites::ListSentServerInvitesResponse, list_server_channels::ListServerChannelsResponse, list_server_members::ListServerMembersResponse, list_servers_by_user::ListServersByUserResponse, }};
use shared::api_error::ApiError;

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
            muted_by: r.muted_by.map(|u| u.uuid()),
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
    muted_by: Option<Uuid>,
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
    viewer_session_id: String,
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
            messaging::dto::requests::channel::update_stream_viewer::UpdateStreamViewerRequest {
                channel_id: body.channel_id,
                streamer_id: body.streamer_id,
                viewer_session_id: body.viewer_session_id,
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

#[delete("/{server_id}/members/{member_id}")]
async fn kick_member(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, ApiError> {
    let actor = UserId::new(auth_token.id);
    let (server_id, member_id) = path.into_inner();
    app_state
        .kick_member_usecase
        .execute(ServerId::new(server_id), actor, UserId::new(member_id))
        .await?;
    Ok(HttpResponse::NoContent().finish())
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

#[derive(Debug, serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
struct CreateInviteCodeBody {
    #[validate(range(min = 1, max = 365))]
    expires_days: Option<i64>,
    #[validate(range(min = 1, max = 10000))]
    max_uses: Option<i32>,
}

#[post("/{server_id}/invite-codes")]
async fn create_invite_code(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<CreateInviteCodeBody>,
) -> Result<impl Responder, ApiError> {
    let body = body.into_inner();
    body.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);

    let invite = app_state
        .create_invite_code_usecase
        .execute(server_id, user_id, body.expires_days, body.max_uses)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(InviteCodeResponse::from(invite))))
}

#[get("/{server_id}/invite-codes")]
async fn list_invite_codes(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);

    let invites = app_state
        .list_invite_codes_usecase
        .execute(server_id, user_id)
        .await?;

    let response = ListInviteCodesResponse {
        invites: invites.into_iter().map(InviteCodeResponse::from).collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[delete("/{server_id}/invite-codes/{code}")]
async fn revoke_invite_code(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, String)>,
) -> Result<impl Responder, ApiError> {
    let (server_id, code) = path.into_inner();
    let server_id = ServerId::new(server_id);
    let user_id = UserId::new(auth_token.id);

    app_state
        .revoke_invite_code_usecase
        .execute(server_id, code, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/invite-codes/{code}/redeem")]
async fn redeem_invite_code(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, ApiError> {
    let code = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let server = app_state
        .redeem_invite_code_usecase
        .execute(code, user_id)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(CreateServerResponse::from(server))))
}

#[patch("/{server_id}")]
async fn update_server(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateServerRequestBody>,
) -> Result<impl Responder, ApiError> {
    let body: UpdateServerRequestBody = body.into_inner();
    body.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let owner_id = UserId::new(auth_token.id);

    app_state
        .update_server_usecase
        .execute(body.into_request(server_id, owner_id, request_context(&req)))
        .await?;

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
