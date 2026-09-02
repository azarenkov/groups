use actix_web::{HttpResponse, Responder, get, patch, web};
use api_common::api_error::StatusCode;
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use identity::usecases::user::appearance::{AppearancePatch, AppearanceSettings};
use identity_domain::user::id::UserId;
use serde::{Deserialize, Serialize};

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceResponse {
    theme: String,
    accent: String,
    chat_font_size: i16,
    reduce_motion: bool,
}

impl From<AppearanceSettings> for AppearanceResponse {
    fn from(settings: AppearanceSettings) -> Self {
        Self {
            theme: settings.theme,
            accent: settings.accent,
            chat_font_size: settings.chat_font_size,
            reduce_motion: settings.reduce_motion,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceBody {
    theme: Option<String>,
    accent: Option<String>,
    chat_font_size: Option<i16>,
    reduce_motion: Option<bool>,
}

#[get("/me/appearance")]
pub async fn get_me_appearance(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let settings = app_state
        .get_appearance_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    match settings {
        Some(settings) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(AppearanceResponse::from(settings))))
        }
        None => Err(HttpError::new(
            "Appearance settings are not set".to_string(),
            StatusCode::NOT_FOUND,
            "APPEARANCE_NOT_SET",
        )),
    }
}

#[patch("/me/appearance")]
pub async fn update_me_appearance(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<AppearanceBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let settings = app_state
        .update_appearance_usecase
        .execute(
            UserId::new(auth_token.id),
            AppearancePatch {
                theme: body.theme,
                accent: body.accent,
                chat_font_size: body.chat_font_size,
                reduce_motion: body.reduce_motion,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(AppearanceResponse::from(settings))))
}
