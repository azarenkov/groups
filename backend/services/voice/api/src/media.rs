use actix_web::{HttpResponse, Responder, get, put, web};
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use domain::user::id::UserId;
use serde::{Deserialize, Serialize};
use voice_domain::media::{MediaRevision, MediaState};

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaStateDto {
    mic_muted: bool,
    deafened: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveMediaStateBody {
    mic_muted: bool,
    deafened: bool,
    revision: Option<RevisionDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevisionDto {
    source: uuid::Uuid,
    seq: i64,
}

impl From<MediaState> for MediaStateDto {
    fn from(state: MediaState) -> Self {
        Self { mic_muted: state.mic_muted(), deafened: state.deafened() }
    }
}

#[get("/media")]
async fn get_media_state(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let state = app_state
        .get_media_state_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(MediaStateDto::from(state))))
}

#[put("/media")]
async fn save_media_state(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<SaveMediaStateBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let state = app_state
        .save_media_state_usecase
        .execute(
            UserId::new(auth_token.id),
            body.mic_muted,
            body.deafened,
            body.revision.map(|r| MediaRevision { source: r.source, seq: r.seq }),
            chrono::Utc::now(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(MediaStateDto::from(state))))
}

pub fn media_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/v1/voice").service(get_media_state).service(save_media_state));
}
