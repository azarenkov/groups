use actix_web::{HttpResponse, Responder, get, web};
use api_common::{dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
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
