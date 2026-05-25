use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;
use shared::middleware::auth::RateLimitedAuthToken;

use crate::shared::{error::ApiError, state::AppState};

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
    let payload = serde_json::to_vec(&serde_json::json!({
        "userId": user_id,
        "appearOffline": body.appear_offline,
    }))
    .unwrap_or_default();

    if let Err(e) = app_state
        .nats_core
        .publish(
            app_state.presence_preference_subject.clone(),
            payload.into(),
        )
        .await
    {
        tracing::warn!(error = %e, %user_id, "Failed to publish presence preference");
    }

    Ok(HttpResponse::NoContent().finish())
}

pub fn presence_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(set_presence);
}
