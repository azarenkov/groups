use actix_web::{HttpResponse, Responder, post, web};
use domain::value_objects::user::id::UserId;

use crate::{
    middleware::auth::AuthToken,
    shared::{
        dto::{
            requests::{create_room::CreateRoomRequest, genereate_token::GenerateTokenRequest},
            responses::{
                create_room::CreateRoomResponse, generate_token::GenerateTokenResponse,
                response::ApiResponse,
            },
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

    let user_id = UserId::new(auth_token.id);

    let room = app_state
        .create_room_usecase()
        .execute(request.into(), user_id)
        .await?;

    let response = CreateRoomResponse::from(room);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[post("/generate_token")]
async fn generate_token(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<GenerateTokenRequest>,
) -> Result<impl Responder, ApiError> {
    let request: GenerateTokenRequest = request.into_inner();

    let user_id = UserId::new(auth_token.id);

    let token = app_state
        .generate_token_usecase()
        .execute(request.into(), user_id)
        .await?;

    let response = GenerateTokenResponse::from(token);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}
