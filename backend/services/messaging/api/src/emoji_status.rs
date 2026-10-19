use actix_web::{HttpResponse, Responder, post, web};
use api_common::{dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::emoji_status::{EmojiStatusRepository, MAX_EMOJI_STATUS_LOOKUP};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusesBody {
    user_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusItem {
    user_id: Uuid,
    emoji_status: Option<EmojiStatusValue>,
    emoji_status_updated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusValue {
    sticker_id: Uuid,
    until: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusesResponse {
    badges: Vec<EmojiStatusItem>,
}

#[post("/badges")]
async fn lookup_emoji_statuses(
    _auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<EmojiStatusesBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    if body.user_ids.len() > MAX_EMOJI_STATUS_LOOKUP {
        return Err(HttpError::bad_request("Too many user ids".to_string()));
    }
    let ids: Vec<UserId> = body.user_ids.into_iter().map(UserId::new).collect();
    let statuses = app_state
        .emoji_status_repository
        .emoji_statuses(&ids, Utc::now())
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(EmojiStatusesResponse {
        badges: statuses
            .into_iter()
            .map(|s| EmojiStatusItem {
                user_id: s.user_id.uuid(),
                emoji_status: s.emoji_status.map(|status| EmojiStatusValue {
                    sticker_id: status.sticker_id,
                    until: status.until,
                }),
                emoji_status_updated_at: s.updated_at,
            })
            .collect(),
    })))
}

pub fn emoji_status_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(lookup_emoji_statuses);
}
