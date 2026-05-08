use actix_web::{HttpResponse, Responder, post, web};

use crate::shared::{
    dto::requests::room::webhook::LiveKitWebhookRequest, error::ApiError,
    mappers::room_event::RoomEventMapper, state::AppState,
};

#[post("/livekit")]
async fn livekit_webhook(
    app_state: web::Data<AppState>,
    request: web::Json<LiveKitWebhookRequest>,
) -> Result<impl Responder, ApiError> {
    let dto = RoomEventMapper::to_application_dto(&request.event, request.data.clone())?;

    app_state.publish_room_event_usecase().execute(dto).await?;

    Ok(HttpResponse::Ok().finish())
}
