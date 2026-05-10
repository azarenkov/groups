use actix_web::{HttpResponse, Responder, http::StatusCode, post, web};
use application::usecases::auth::login::LoginError;
use shared::jwt::validation::{encode_token_pair, validate_refresh_token};
use validator::Validate;

use crate::shared::{
    dto::{
        requests::auth::{
            login::LoginUserRequestBody, refresh_token::RefreshTokenRequestBody,
            registration::RegisterUserRequestBody,
        },
        responses::{ApiResponse, auth::registration::RegisterUserResponse},
    },
    error::ApiError,
    state::AppState,
};

#[post("/register")]
async fn register_user(
    app_state: web::Data<AppState>,
    body: web::Json<RegisterUserRequestBody>,
) -> Result<impl Responder, ApiError> {
    let body: RegisterUserRequestBody = body.into_inner();
    body.validate()?;

    let user = app_state.register_user_usecase.execute(body.into()).await?;

    let response = RegisterUserResponse::from(user);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[post("/login")]
async fn login_user(
    app_state: web::Data<AppState>,
    request: web::Json<LoginUserRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: LoginUserRequestBody = request.into_inner();
    request.validate()?;

    let user_id = app_state.login_user_usecase.execute(request.into()).await?;

    let token_pair = encode_token_pair(user_id.uuid(), app_state.jwt_secret.as_str())
        .await
        .map_err(|_| ApiError::from(LoginError::Internal))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(token_pair)))
}

#[post("/refresh")]
async fn refresh_tokens(
    app_state: web::Data<AppState>,
    body: web::Json<RefreshTokenRequestBody>,
) -> Result<impl Responder, ApiError> {
    let body: RefreshTokenRequestBody = body.into_inner();

    let user_uuid = validate_refresh_token(&body.refresh_token, app_state.jwt_secret.as_str())
        .await
        .map_err(|_| {
            ApiError::new(
                "Invalid refresh token".to_string(),
                StatusCode::UNAUTHORIZED,
            )
        })?;

    let token_pair = encode_token_pair(user_uuid, app_state.jwt_secret.as_str())
        .await
        .map_err(|_| {
            ApiError::new(
                "Failed to encode tokens".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(token_pair)))
}
