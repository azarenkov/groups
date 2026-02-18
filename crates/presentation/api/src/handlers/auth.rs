use actix_web::{HttpResponse, Responder, post, web};
use application::usecases::auth::login::LoginError;
use shared::jwt::validation::encode_token_pair;
use validator::Validate;

use crate::shared::{
    dto::{
        requests::auth::{login::LoginUserRequest, registration::RegisterUserRequest},
        responses::{ApiResponse, auth::registration::RegisterUserResponse},
    },
    error::ApiError,
    state::AppState,
};

#[post("/register")]
async fn register_user(
    app_state: web::Data<AppState>,
    request: web::Json<RegisterUserRequest>,
) -> Result<impl Responder, ApiError> {
    let request: RegisterUserRequest = request.into_inner();
    request.validate()?;

    let user = app_state
        .register_user_usecase()
        .execute(request.into())
        .await?;

    let response = RegisterUserResponse::from(user);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[post("/login")]
async fn login_user(
    app_state: web::Data<AppState>,
    request: web::Json<LoginUserRequest>,
) -> Result<impl Responder, ApiError> {
    let request: LoginUserRequest = request.into_inner();
    request.validate()?;

    let user_id = app_state
        .login_user_usecase()
        .execute(request.into())
        .await?;

    let token_pair = encode_token_pair(user_id.uuid(), app_state.jwt_secret())
        .await
        .map_err(|_| ApiError::from(LoginError::Internal))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(token_pair)))
}
