use actix_web::{HttpRequest, HttpResponse, Responder, post, web};

use crate::shared::{dto::responses::ApiResponse, error::ApiError, state::AppState};

#[post("/livekit")]
async fn livekit_webhook(
    app_state: web::Data<AppState>,
    payload: web::Bytes,
) -> Result<impl Responder, ApiError> {
    let payload_str = String::from_utf8_lossy(&payload);

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload_str) {
        println!("\nParsed JSON ");
        println!(
            "{}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
    }

    Ok(HttpResponse::Ok())
}
