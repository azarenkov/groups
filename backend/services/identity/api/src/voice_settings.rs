use actix_web::{HttpResponse, Responder, get, patch, web};
use api_common::api_error::StatusCode;
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use identity::usecases::user::voice_settings::{VoiceSettings, VoiceSettingsPatch};
use identity_domain::user::id::UserId;
use serde::{Deserialize, Serialize};

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceSettingsResponse {
    mic_volume: i16,
    output_volume: i16,
    noise_suppression: bool,
    auto_sensitivity: bool,
    sensitivity: i16,
    push_to_talk: bool,
}

impl From<VoiceSettings> for VoiceSettingsResponse {
    fn from(settings: VoiceSettings) -> Self {
        Self {
            mic_volume: settings.mic_volume,
            output_volume: settings.output_volume,
            noise_suppression: settings.noise_suppression,
            auto_sensitivity: settings.auto_sensitivity,
            sensitivity: settings.sensitivity,
            push_to_talk: settings.push_to_talk,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VoiceSettingsBody {
    mic_volume: Option<i16>,
    output_volume: Option<i16>,
    noise_suppression: Option<bool>,
    auto_sensitivity: Option<bool>,
    sensitivity: Option<i16>,
    push_to_talk: Option<bool>,
}

#[get("/me/voice")]
pub async fn get_me_voice_settings(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let settings = app_state
        .get_voice_settings_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    match settings {
        Some(settings) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            VoiceSettingsResponse::from(settings),
        ))),
        None => Err(HttpError::new(
            "Voice settings are not set".to_string(),
            StatusCode::NOT_FOUND,
            "VOICE_SETTINGS_NOT_SET",
        )),
    }
}

#[patch("/me/voice")]
pub async fn update_me_voice_settings(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<VoiceSettingsBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let settings = app_state
        .update_voice_settings_usecase
        .execute(
            UserId::new(auth_token.id),
            VoiceSettingsPatch {
                mic_volume: body.mic_volume,
                output_volume: body.output_volume,
                noise_suppression: body.noise_suppression,
                auto_sensitivity: body.auto_sensitivity,
                sensitivity: body.sensitivity,
                push_to_talk: body.push_to_talk,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(VoiceSettingsResponse::from(settings))))
}
