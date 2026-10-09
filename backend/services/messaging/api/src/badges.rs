use actix_web::{HttpResponse, Responder, post, web};
use api_common::{dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::badge::{BadgeRepository, MAX_BADGE_LOOKUP};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BadgesBody {
    user_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BadgeItem {
    user_id: Uuid,
    tier: String,
    badge_style: String,
    member_number: Option<i64>,
    emoji_status: Option<EmojiStatusItem>,
    emoji_status_updated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusItem {
    sticker_id: Uuid,
    until: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BadgesResponse {
    badges: Vec<BadgeItem>,
}

#[post("/badges")]
async fn lookup_badges(
    _auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<BadgesBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    if body.user_ids.len() > MAX_BADGE_LOOKUP {
        return Err(HttpError::bad_request("Too many user ids".to_string()));
    }
    let ids: Vec<UserId> = body.user_ids.into_iter().map(UserId::new).collect();
    let badges = app_state
        .badge_repository
        .paid_badges(&ids, Utc::now())
        .await
        .map_err(HttpError::from)?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(BadgesResponse {
        badges: badges
            .into_iter()
            .map(|b| BadgeItem {
                user_id: b.user_id.uuid(),
                tier: b.tier,
                badge_style: b.badge_style,
                member_number: b.member_number,
                emoji_status: b.emoji_status.map(|status| EmojiStatusItem {
                    sticker_id: status.sticker_id,
                    until: status.until,
                }),
                emoji_status_updated_at: b.emoji_status_updated_at,
            })
            .collect(),
    })))
}

pub fn badge_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(lookup_badges);
}
