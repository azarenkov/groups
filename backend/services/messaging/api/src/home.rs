use actix_web::{HttpResponse, Responder, get, web};
use api_common::{dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::{friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, privacy::ContactPolicy, privacy::PrivacyRepository};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ActiveFriendItem {
    user_id: Uuid,
    user_name: String,
    channel_id: Uuid,
    channel_name: String,
    server_id: Uuid,
    server_name: String,
    joined_at: DateTime<Utc>,
    is_muted: bool,
    camera_on: bool,
    screen_on: bool,
    profile_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentRoomItem {
    channel_id: Uuid,
    channel_name: String,
    server_id: Uuid,
    server_name: String,
    live_count: i64,
    last_join: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HomeOverviewResponse {
    active_friends: Vec<ActiveFriendItem>,
    recent_rooms: Vec<RecentRoomItem>,
}

#[get("/me/home")]
async fn get_home_overview(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let overview = app_state
        .get_home_overview_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    let response = HomeOverviewResponse {
        active_friends: overview
            .active_friends
            .into_iter()
            .map(|r| ActiveFriendItem {
                user_id: r.user_id.uuid(),
                user_name: r.user_name,
                channel_id: r.channel_id.uuid(),
                channel_name: r.channel_name,
                server_id: r.server_id.uuid(),
                server_name: r.server_name,
                joined_at: r.joined_at,
                is_muted: r.is_muted,
                camera_on: r.camera_on,
                screen_on: r.screen_on,
                profile_updated_at: r.profile_updated_at,
            })
            .collect(),
        recent_rooms: overview
            .recent_rooms
            .into_iter()
            .map(|r| RecentRoomItem {
                channel_id: r.channel_id.uuid(),
                channel_name: r.channel_name,
                server_id: r.server_id.uuid(),
                server_name: r.server_name,
                live_count: r.live_count,
                last_join: r.last_join,
            })
            .collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MutualServerItem {
    id: Uuid,
    name: String,
    accent_color: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceRoomItem {
    channel_id: Uuid,
    channel_name: String,
    server_id: Uuid,
    server_name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicProfileResponse {
    id: Uuid,
    name: String,
    bio: String,
    profile_updated_at: Option<DateTime<Utc>>,
    active: bool,
    timezone: String,
    member_since: Option<DateTime<Utc>>,
    accepts_calls: bool,
    accepts_friend_requests: bool,
    mutual_servers: Vec<MutualServerItem>,
    voice_room: Option<VoiceRoomItem>,
}

#[get("/{user_id}/profile")]
async fn get_user_profile(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let target_id = path.into_inner();
    let view = app_state
        .get_user_profile_usecase
        .execute(UserId::new(auth_token.id), UserId::new(target_id))
        .await?;
    let settings = app_state
        .privacy_repository
        .get(&UserId::new(target_id))
        .await
        .map_err(HttpError::from)?;
    let accepts_calls = match settings.call_policy {
        ContactPolicy::All => true,
        ContactPolicy::NoOne => false,
        ContactPolicy::Friends => {
            let pair = app_state
                .friendship_repository
                .find_pair(&UserId::new(auth_token.id), &UserId::new(target_id))
                .await
                .map_err(HttpError::from)?;
            matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted)
        }
    };
    let accepts_friend_requests = settings.accept_friend_requests;
    Ok(HttpResponse::Ok().json(ApiResponse::success(PublicProfileResponse {
        id: view.profile.user_id.uuid(),
        name: view.profile.name,
        bio: view.profile.bio,
        profile_updated_at: view.profile.profile_updated_at,
        active: view.profile.active,
        timezone: view.profile.timezone,
        member_since: view.profile.registered_at,
        accepts_calls,
        accepts_friend_requests,
        mutual_servers: view
            .mutual_servers
            .into_iter()
            .map(|m| MutualServerItem {
                id: m.server_id.uuid(),
                name: m.name,
                accent_color: m.accent_color,
            })
            .collect(),
        voice_room: view.voice_room.map(|v| VoiceRoomItem {
            channel_id: v.channel_id.uuid(),
            channel_name: v.channel_name,
            server_id: v.server_id.uuid(),
            server_name: v.server_name,
        }),
    })))
}
