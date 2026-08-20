use actix_web::{HttpResponse, Responder, get, post, web};
use chrono::{DateTime, Utc};
use domain::{channel::id::ChannelId, user::id::UserId};
use serde::Serialize;
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::messaging::state::AppState;
use crate::shared::{
    dto::responses::ApiResponse, error::ApiError, };

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
    server_id: Uuid,
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
                server_id: c.server_id,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DmReadReceiptResponse {
    read_at: Option<DateTime<Utc>>,
}

#[get("/dms/{friend_id}/read-state")]
async fn get_dm_read_receipt(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let friend_id = UserId::new(path.into_inner());
    let read_at = app_state
        .get_dm_read_receipt_usecase
        .execute(user_id, friend_id)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(DmReadReceiptResponse { read_at })))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemberReadItem {
    user_id: Uuid,
    read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ChannelReadReceiptsResponse {
    members: Vec<MemberReadItem>,
}

#[get("/channels/{channel_id}/read-states")]
async fn get_channel_read_receipts(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let channel_id = ChannelId::new(path.into_inner());
    let reads = app_state
        .get_channel_read_receipts_usecase
        .execute(user_id, channel_id)
        .await?;
    let response = ChannelReadReceiptsResponse {
        members: reads
            .into_iter()
            .map(|m| MemberReadItem {
                user_id: m.user_id,
                read_at: m.read_at,
            })
            .collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

pub fn read_state_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(mark_channel_read);
    cfg.service(mark_dm_read);
    cfg.service(get_unread);
    cfg.service(get_dm_read_receipt);
    cfg.service(get_channel_read_receipts);
}
