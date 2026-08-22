use actix_web::{HttpResponse, Responder, get, put, web};
use domain::{
    notification::{NotificationLevel, NotificationScope},
    user::id::UserId,
};
use serde::{Deserialize, Serialize};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::state::AppState;
use api_common::dto::responses::ApiResponse;
use shared::api_error::ApiError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreferenceItem {
    scope_type: String,
    scope_id: Uuid,
    level: String,
}

#[derive(Debug, Serialize)]
struct PreferencesResponse {
    preferences: Vec<PreferenceItem>,
}

#[get("/notifications/preferences")]
async fn list_notification_preferences(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let preferences = app_state
        .list_notification_preferences_usecase
        .execute(user_id)
        .await?;
    let response = PreferencesResponse {
        preferences: preferences
            .into_iter()
            .map(|p| PreferenceItem {
                scope_type: p.scope.as_str().to_string(),
                scope_id: p.scope_id,
                level: p.level.as_str().to_string(),
            })
            .collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetPreferenceRequest {
    scope_type: String,
    scope_id: Uuid,
    level: String,
}

#[put("/notifications/preferences")]
async fn set_notification_preference(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<SetPreferenceRequest>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let scope = NotificationScope::parse(&body.scope_type).ok_or_else(|| {
        ApiError::bad_request("Unknown notification scope".to_string())
    })?;
    let level = NotificationLevel::parse(&body.level).ok_or_else(|| {
        ApiError::bad_request("Unknown notification level".to_string())
    })?;
    app_state
        .set_notification_preference_usecase
        .execute(user_id, scope, body.scope_id, level)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn notification_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_notification_preferences);
    cfg.service(set_notification_preference);
}
