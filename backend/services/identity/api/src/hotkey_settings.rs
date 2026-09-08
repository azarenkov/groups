use std::collections::BTreeMap;

use actix_web::{HttpResponse, Responder, get, patch, web};
use api_common::api_error::StatusCode;
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use identity_domain::user::settings::hotkeys::{HotkeyBindings, HotkeyPatch};
use identity_domain::user::id::UserId;
use serde::{Deserialize, Serialize};

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotkeySettingsResponse {
    bindings: BTreeMap<String, String>,
}

impl From<HotkeyBindings> for HotkeySettingsResponse {
    fn from(settings: HotkeyBindings) -> Self {
        Self {
            bindings: settings
                .overrides()
                .iter()
                .map(|(action, binding)| (action.id().to_string(), binding.as_str().to_string()))
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HotkeySettingsBody {
    #[serde(default)]
    bindings: BTreeMap<String, Option<String>>,
}

#[get("/me/hotkeys")]
pub async fn get_me_hotkey_settings(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let settings = app_state
        .get_hotkey_settings_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    match settings {
        Some(settings) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            HotkeySettingsResponse::from(settings),
        ))),
        None => Err(HttpError::new(
            "Hotkey settings are not set".to_string(),
            StatusCode::NOT_FOUND,
            "HOTKEY_SETTINGS_NOT_SET",
        )),
    }
}

#[patch("/me/hotkeys")]
pub async fn update_me_hotkey_settings(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<HotkeySettingsBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let settings = app_state
        .update_hotkey_settings_usecase
        .execute(
            UserId::new(auth_token.id),
            HotkeyPatch {
                bindings: body.bindings,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        HotkeySettingsResponse::from(settings),
    )))
}
