use actix_web::{HttpResponse, Responder, delete, post, web};
use domain::{room::value_objects::id::RoomId, user::value_objects::id::UserId};
use uuid::Uuid;
use validator::Validate;

use crate::{
    middleware::auth::AuthToken,
    shared::{
        dto::{
            requests::room::create_room::CreateRoomRequest,
            responses::{ApiResponse, room::create_room::CreateRoomResponse},
        },
        error::ApiError,
        state::AppState,
    },
};

#[post("/create")]
async fn create_room(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<CreateRoomRequest>,
) -> Result<impl Responder, ApiError> {
    let request: CreateRoomRequest = request.into_inner();
    request.validate()?;

    let user_id = UserId::new(auth_token.id);

    let room = app_state
        .create_room_usecase()
        .execute(request.into(), user_id)
        .await?;

    let response = CreateRoomResponse::from(room);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[delete("/{room_id}")]
async fn delete_room(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let room_id = RoomId::new(path.into_inner());
    let owner_id = UserId::new(auth_token.id);

    let deleted = app_state
        .delete_room_usecase()
        .execute(room_id, owner_id)
        .await?;

    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
