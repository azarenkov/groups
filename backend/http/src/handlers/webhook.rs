use actix_web::{HttpResponse, Responder, post, web};

use crate::shared::{
    dto::requests::channel::webhook::LiveKitWebhookRequestBody, error::ApiError, state::AppState,
};

#[post("/livekit")]
async fn livekit_webhook(
    app_state: web::Data<AppState>,
    request: web::Json<LiveKitWebhookRequestBody>,
) -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().finish())
}
