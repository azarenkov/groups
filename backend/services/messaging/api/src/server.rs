use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, put, web};
use messaging::dto::requests::server::delete_server::DeleteServerRequest;
use messaging_domain::server::id::ServerId;
use messaging_domain::server::invite_link::InviteToken;
use api_common::api_error::StatusCode;
use domain::user::id::UserId;
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;
use api_common::{context::request_context};
use api_common::dto::{cursor::CursorQuery, pagination::{PaginationMeta, PaginationQuery}, responses::ApiResponse};
use crate::dto::{requests::server::{ create_server::CreateServerRequestBody, update_server::UpdateServerRequestBody, }, responses::server::{ create_server::CreateServerResponse, invite_links::{InviteLinkResponse, ListInviteLinksResponse}, list_sent_server_invites::ListSentServerInvitesResponse, list_server_channels::ListServerChannelsResponse, list_server_members::ListServerMembersResponse, list_servers_by_user::ListServersByUserResponse, }};
use crate::http_errors::HttpError;

#[post("")]
async fn create_server(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<CreateServerRequestBody>,
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
    query.validate()?;

    let user_id = UserId::new(auth_token.id);
    let pagination = api_common::pagination::parse(query.page, query.per_page)?;

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
) -> Result<impl Responder, HttpError> {
    query.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let requester_id = UserId::new(auth_token.id);
    let pagination = api_common::pagination::parse(query.page, query.per_page)?;

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
) -> Result<impl Responder, HttpError> {
    query.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let requester_id = UserId::new(auth_token.id);
    let pagination = api_common::pagination::parse(query.page, query.per_page)?;

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
) -> Result<impl Responder, HttpError> {
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
            deafened_by: r.deafened_by.map(|u| u.uuid()),
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
    deafened_by: Option<Uuid>,
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);

    app_state.leave_server_usecase.execute(server_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
struct CreateInviteLinkBody {
    #[validate(length(min = 1, max = 60))]
    name: Option<String>,
    #[validate(range(min = 1, max = 365))]
    expires_days: Option<i64>,
    #[validate(range(min = 1, max = 10000))]
    max_uses: Option<i32>,
}

fn parse_invite_token(raw: &str) -> Result<InviteToken, HttpError> {
    InviteToken::parse(raw).ok_or_else(|| {
        HttpError::new(
            "Invite link not found.".to_string(),
            StatusCode::NOT_FOUND,
            "INVITE_NOT_FOUND",
        )
    })
}

#[post("/{server_id}/invite-links")]
async fn create_invite_link(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<CreateInviteLinkBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);
    let name = body.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());

    let link = app_state
        .create_invite_link_usecase
        .execute(server_id, user_id, name, body.expires_days, body.max_uses)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(InviteLinkResponse::from_link(
        link,
        &app_state.public_app_url,
    ))))
}

#[get("/{server_id}/invite-links")]
async fn list_invite_links(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<CursorQuery>,
) -> Result<impl Responder, HttpError> {
    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);
    let (cursor, limit) = query.keyset().map_err(|_| {
        HttpError::new("Invalid cursor".to_string(), StatusCode::BAD_REQUEST, "INVALID_CURSOR")
    })?;
    if let Some(c) = &cursor
        && InviteToken::parse(&c.key).is_none()
    {
        return Err(HttpError::new("Invalid cursor".to_string(), StatusCode::BAD_REQUEST, "INVALID_CURSOR"));
    }

    let result = app_state
        .list_invite_links_usecase
        .execute(server_id, user_id, cursor, limit)
        .await?;

    let response = ListInviteLinksResponse {
        primary: InviteLinkResponse::from_link(result.primary, &app_state.public_app_url),
        next_cursor: result.page.next_cursor.as_ref().map(|c| c.encode()),
        links: result
            .page
            .items
            .into_iter()
            .map(|link| InviteLinkResponse::from_link(link, &app_state.public_app_url))
            .collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[delete("/{server_id}/invite-links/{token}")]
async fn revoke_invite_link(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, String)>,
) -> Result<impl Responder, HttpError> {
    let (server_id, token) = path.into_inner();
    let token = parse_invite_token(&token)?;
    let server_id = ServerId::new(server_id);
    let user_id = UserId::new(auth_token.id);

    app_state
        .revoke_invite_link_usecase
        .execute(server_id, token, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[patch("/{server_id}")]
async fn update_server(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateServerRequestBody>,
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
