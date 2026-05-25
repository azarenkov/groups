use actix_web::{HttpResponse, Responder, get, post, web};
use domain::{channel::id::ChannelId, user::id::UserId};
use serde::Serialize;
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::shared::{
    dto::responses::ApiResponse, error::ApiError, state::AppState,
};

#[post("/channels/{channel_id}/read")]
async fn mark_channel_read(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let channel_id = ChannelId::new(path.into_inner());
    app_state
        .mark_channel_read_usecase
        .execute(user_id, channel_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/dms/{friend_id}/read")]
async fn mark_dm_read(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let friend_id = UserId::new(path.into_inner());
    app_state
        .mark_dm_read_usecase
        .execute(user_id, friend_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnreadChannelItem {
    channel_id: Uuid,
    count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnreadDmItem {
    friend_id: Uuid,
    count: i64,
}

#[derive(Debug, Serialize)]
struct UnreadResponse {
    channels: Vec<UnreadChannelItem>,
    dms: Vec<UnreadDmItem>,
}

#[get("/me/unread")]
async fn get_unread(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let counts = app_state.get_unread_counts_usecase.execute(user_id).await?;
    let response = UnreadResponse {
        channels: counts
            .channels
            .into_iter()
            .map(|c| UnreadChannelItem {
                channel_id: c.channel_id,
                count: c.count,
            })
            .collect(),
        dms: counts
            .dms
            .into_iter()
            .map(|d| UnreadDmItem {
                friend_id: d.friend_id,
                count: d.count,
            })
            .collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

pub fn read_state_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(mark_channel_read);
    cfg.service(mark_dm_read);
    cfg.service(get_unread);
}
