use actix_web::{HttpResponse, Responder, get, patch, web};
use api_common::{dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use domain::user::id::UserId;
use messaging::usecases::privacy::update_privacy::PrivacyPatch;
use messaging_domain::privacy::PrivacySettings;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivacyResponse {
    dm_policy: &'static str,
    call_policy: &'static str,
    accept_friend_requests: bool,
}

impl From<PrivacySettings> for PrivacyResponse {
    fn from(settings: PrivacySettings) -> Self {
        Self {
            dm_policy: settings.dm_policy.as_str(),
            call_policy: settings.call_policy.as_str(),
            accept_friend_requests: settings.accept_friend_requests,
        }
    }
}

#[get("/me/privacy")]
async fn get_privacy(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let settings = app_state
        .get_privacy_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(PrivacyResponse::from(settings))))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePrivacyBody {
    dm_policy: Option<String>,
    call_policy: Option<String>,
    accept_friend_requests: Option<bool>,
}

#[patch("/me/privacy")]
async fn update_privacy(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<UpdatePrivacyBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let settings = app_state
        .update_privacy_usecase
        .execute(
            UserId::new(auth_token.id),
            PrivacyPatch {
                dm_policy: body.dm_policy,
                call_policy: body.call_policy,
                accept_friend_requests: body.accept_friend_requests,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(PrivacyResponse::from(settings))))
}

pub fn privacy_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_privacy).service(update_privacy);
}
