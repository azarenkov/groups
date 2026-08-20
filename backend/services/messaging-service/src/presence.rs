use actix_web::{HttpResponse, Responder, post, web};
use contracts::realtime::PresencePreferenceV1;
use serde::Deserialize;
use shared::middleware::auth::RateLimitedAuthToken;

use crate::state::AppState;
use api_common::{error::ApiError};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceBody {
    pub appear_offline: bool,
}

#[post("/me/presence")]
async fn set_presence(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<PresenceBody>,
) -> Result<impl Responder, ApiError> {
    let user_id = auth_token.id;
    let payload = serde_json::to_vec(&PresencePreferenceV1 {
        user_id,
        appear_offline: body.appear_offline,
    })
    .unwrap_or_default();

    app_state
        .nats_core
        .publish(
            app_state.presence_preference_subject.clone(),
            payload.into(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %user_id, "Failed to publish presence preference");
            ApiError::new(
                "Failed to publish presence preference".to_string(),
                actix_web::http::StatusCode::BAD_GATEWAY,
                "PRESENCE_PUBLISH_FAILED",
            )
        })?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn presence_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(set_presence);
}
