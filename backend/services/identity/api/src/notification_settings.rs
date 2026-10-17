use actix_web::{HttpResponse, Responder, get, patch, web};
use api_common::api_error::StatusCode;
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use identity_domain::user::settings::notification::{
    NotificationSettings, NotificationSettingsPatch,
};
use identity_domain::user::id::UserId;
use serde::{Deserialize, Serialize};

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotificationSettingsResponse {
    desktop_enabled: bool,
    sounds_enabled: bool,
    mentions: String,
    quiet_hours_enabled: bool,
    quiet_start: String,
    quiet_end: String,
    calls_enabled: bool,
    voice_cues_enabled: bool,
}

impl From<NotificationSettings> for NotificationSettingsResponse {
    fn from(settings: NotificationSettings) -> Self {
        Self {
            desktop_enabled: settings.desktop_enabled(),
            sounds_enabled: settings.sounds_enabled(),
            mentions: settings.mentions().as_str().to_string(),
            quiet_hours_enabled: settings.quiet_hours_enabled(),
            quiet_start: settings.quiet_start().as_str().to_string(),
            quiet_end: settings.quiet_end().as_str().to_string(),
            calls_enabled: settings.calls_enabled(),
            voice_cues_enabled: settings.voice_cues_enabled(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationSettingsBody {
    desktop_enabled: Option<bool>,
    sounds_enabled: Option<bool>,
    mentions: Option<String>,
    quiet_hours_enabled: Option<bool>,
    quiet_start: Option<String>,
    quiet_end: Option<String>,
    calls_enabled: Option<bool>,
    voice_cues_enabled: Option<bool>,
}

#[get("/me/notifications")]
pub async fn get_me_notification_settings(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let settings = app_state
        .get_notification_settings_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    match settings {
        Some(settings) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            NotificationSettingsResponse::from(settings),
        ))),
        None => Err(HttpError::new(
            "Notification settings are not set".to_string(),
            StatusCode::NOT_FOUND,
            "NOTIFICATION_SETTINGS_NOT_SET",
        )),
    }
}

#[patch("/me/notifications")]
pub async fn update_me_notification_settings(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<NotificationSettingsBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let settings = app_state
        .update_notification_settings_usecase
        .execute(
            UserId::new(auth_token.id),
            NotificationSettingsPatch {
                desktop_enabled: body.desktop_enabled,
                sounds_enabled: body.sounds_enabled,
                mentions: body.mentions,
                quiet_hours_enabled: body.quiet_hours_enabled,
                quiet_start: body.quiet_start,
                quiet_end: body.quiet_end,
                calls_enabled: body.calls_enabled,
                voice_cues_enabled: body.voice_cues_enabled,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        NotificationSettingsResponse::from(settings),
    )))
}
