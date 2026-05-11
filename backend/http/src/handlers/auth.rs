use actix_web::{HttpResponse, Responder, post, web};
use domain::user::id::UserId;
use shared::middleware::auth::AuthToken;
use validator::Validate;

use crate::shared::{
    dto::{
        requests::auth::{
            confirm_registration::ConfirmRegistrationRequestBody, login::LoginUserRequestBody,
            logout::LogoutRequestBody, refresh_token::RefreshTokenRequestBody,
            registration::RegisterUserRequestBody,
        },
        responses::{ApiResponse, auth::token_pair::TokenPairResponse},
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

    let tokens = app_state.register_user_usecase.execute(body.into()).await?;

    let response: TokenPairResponse = tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/confirm_registration")]
async fn confirm_registration(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<ConfirmRegistrationRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: ConfirmRegistrationRequestBody = request.into_inner();
    request.validate()?;
    let user_id = UserId::new(auth_token.id);

    app_state
        .confirm_registration_usecase
        .execute(user_id, request.into())
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/login")]
async fn login_user(
    app_state: web::Data<AppState>,
    request: web::Json<LoginUserRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: LoginUserRequestBody = request.into_inner();
    request.validate()?;

    let tokens = app_state.login_user_usecase.execute(request.into()).await?;

    let response: TokenPairResponse = tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/refresh")]
async fn refresh_tokens(
    app_state: web::Data<AppState>,
    request: web::Json<RefreshTokenRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: RefreshTokenRequestBody = request.into_inner();
    request.validate()?;

    let tokens = app_state
        .refresh_tokens_usecase
        .execute(request.into())
        .await?;

    let response: TokenPairResponse = tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/logout")]
async fn logout(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<LogoutRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: LogoutRequestBody = request.into_inner();
    request.validate()?;
    let user_id = UserId::new(auth_token.id);

    app_state
        .logout_usecase
        .execute(user_id, request.into())
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
