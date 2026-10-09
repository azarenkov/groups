use actix_web::{HttpResponse, Responder, delete, get, patch, post, put, web};
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use chrono::{DateTime, Utc};
use identity_domain::user::emoji_status::EmojiStatus;
use identity_domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use serde::{Deserialize, Serialize};
use contracts::internal::users::TrialStateV1;
use uuid::Uuid;

use api_common::client::extract_client_context;
use api_common::context::request_context;
use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EntitlementsResponse {
    tier: &'static str,
    badge_style: &'static str,
    member_number: Option<i64>,
    valid_until: Option<DateTime<Utc>>,
    trial: bool,
    trial_available: bool,
    emoji_status: Option<EmojiStatusResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusResponse {
    sticker_id: Uuid,
    until: Option<DateTime<Utc>>,
}

impl From<EmojiStatus> for EmojiStatusResponse {
    fn from(status: EmojiStatus) -> Self {
        Self { sticker_id: status.sticker_id(), until: status.until() }
    }
}

#[get("/me/entitlements")]
async fn get_my_entitlements(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let entitlements = app_state
        .get_entitlements_usecase
        .execute(&user_id)
        .await
        .map_err(HttpError::from_error)?;
    let user = app_state
        .user_repository
        .find_by_id(&user_id)
        .await
        .map_err(HttpError::from_error)?;
    let badge_style = user.as_ref().map(|u| u.badge_style().as_str()).unwrap_or("");
    let verified = user.as_ref().is_some_and(|u| u.is_email_verified());
    let emoji_status = user
        .as_ref()
        .and_then(|u| u.active_emoji_status(Utc::now()))
        .map(EmojiStatusResponse::from);
    Ok(HttpResponse::Ok().json(ApiResponse::success(EntitlementsResponse {
        tier: entitlements.tier.as_str(),
        badge_style,
        member_number: entitlements.member_number,
        valid_until: entitlements.valid_until,
        trial: entitlements.trial,
        trial_available: entitlements.trial_available && verified,
        emoji_status,
    })))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrialResponse {
    tier: &'static str,
    valid_until: DateTime<Utc>,
    trial: bool,
}

#[post("/me/trial")]
async fn start_my_trial(
    req: actix_web::HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let ctx = request_context(&req);
    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
    let valid_until = app_state
        .start_trial_usecase
        .execute(
            UserId::new(auth_token.id),
            client.ip.as_deref(),
            ctx.request_id.as_deref(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(TrialResponse {
        tier: "pro",
        valid_until,
        trial: true,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BadgeStyleBody {
    style: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BadgeStyleResponse {
    badge_style: &'static str,
}

#[patch("/me/badge")]
async fn set_my_badge_style(
    req: actix_web::HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<BadgeStyleBody>,
) -> Result<impl Responder, HttpError> {
    let ctx = request_context(&req);
    let style = app_state
        .set_badge_style_usecase
        .execute(
            UserId::new(auth_token.id),
            &body.style,
            ctx.request_id.as_deref(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(BadgeStyleResponse {
        badge_style: style.as_str(),
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusBody {
    sticker_id: Uuid,
    duration: Option<String>,
}

#[put("/me/emoji-status")]
async fn set_my_emoji_status(
    req: actix_web::HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<EmojiStatusBody>,
) -> Result<impl Responder, HttpError> {
    let ctx = request_context(&req);
    let (status, changed_at) = app_state
        .set_emoji_status_usecase
        .set(
            UserId::new(auth_token.id),
            body.sticker_id,
            body.duration.as_deref(),
            ctx.request_id.as_deref(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(EmojiStatusChange {
        emoji_status: Some(EmojiStatusResponse::from(status)),
        updated_at: changed_at,
    })))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmojiStatusChange {
    emoji_status: Option<EmojiStatusResponse>,
    updated_at: DateTime<Utc>,
}

#[delete("/me/emoji-status")]
async fn clear_my_emoji_status(
    req: actix_web::HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let ctx = request_context(&req);
    let changed_at = app_state
        .set_emoji_status_usecase
        .clear(UserId::new(auth_token.id), ctx.request_id.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(EmojiStatusChange {
        emoji_status: None,
        updated_at: changed_at,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrantBody {
    user_id: Uuid,
    tier: String,
    valid_until: Option<DateTime<Utc>>,
}

#[post("/internal/entitlements")]
async fn grant_entitlements(
    app_state: web::Data<AppState>,
    body: web::Json<GrantBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    app_state
        .grant_entitlements_usecase
        .execute(UserId::new(body.user_id), &body.tier, body.valid_until, "manual")
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn entitlement_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_my_entitlements)
        .service(set_my_badge_style)
        .service(set_my_emoji_status)
        .service(clear_my_emoji_status)
        .service(start_my_trial);
}

#[get("/internal/users/{user_id}/trial")]
async fn trial_state(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(path.into_inner());
    let entitlements = app_state
        .get_entitlements_usecase
        .execute(&user_id)
        .await
        .map_err(HttpError::from_error)?;
    let user = app_state
        .user_repository
        .find_by_id(&user_id)
        .await
        .map_err(HttpError::from_error)?
        .filter(|user| !user.is_deleted());
    let on_trial = entitlements.trial && user.is_some();
    Ok(HttpResponse::Ok().json(TrialStateV1 {
        on_trial,
        valid_until: entitlements.valid_until.filter(|_| on_trial),
        email: user.as_ref().map(|u| u.email().as_str().to_string()),
        locale: user.as_ref().map(|u| u.locale().as_str().to_string()),
    }))
}

pub fn internal_entitlement_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(grant_entitlements).service(trial_state);
}
