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
    updated_at: chrono::DateTime<chrono::Utc>,
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
                scope_type: p.preference.scope.as_str().to_string(),
                scope_id: p.preference.scope_id,
                level: p.preference.level.as_str().to_string(),
                updated_at: p.updated_at,
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
    let updated_at = app_state
        .set_notification_preference_usecase
        .execute(user_id, scope, body.scope_id, level)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(SetPreferenceResponse {
        updated_at,
    })))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPreferenceResponse {
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub fn notification_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_notification_preferences);
    cfg.service(set_notification_preference);
}
