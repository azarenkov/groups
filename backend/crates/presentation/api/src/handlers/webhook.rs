use actix_web::{HttpResponse, Responder, post, web};

use crate::shared::{
    dto::requests::channel::webhook::LiveKitWebhookRequest, error::ApiError, state::AppState,
};

#[post("/livekit")]
async fn livekit_webhook(
    app_state: web::Data<AppState>,
    request: web::Json<LiveKitWebhookRequest>,
) -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().finish())
}
