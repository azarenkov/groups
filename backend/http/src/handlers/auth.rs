use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use domain::user::id::UserId;
use shared::middleware::auth::AuthToken;
use validator::Validate;

use crate::shared::{
    client::extract_client_context,
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
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<RegisterUserRequestBody>,
) -> Result<impl Responder, ApiError> {
    let body: RegisterUserRequestBody = body.into_inner();
    body.validate()?;

    let client = extract_client_context(&req);
    let tokens = app_state
        .register_user_usecase
        .execute(body.into_request(client))
        .await?;

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
    req: HttpRequest,
    app_state: web::Data<AppState>,
    request: web::Json<LoginUserRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: LoginUserRequestBody = request.into_inner();
    request.validate()?;

    let client = extract_client_context(&req);
    let tokens = app_state
        .login_user_usecase
        .execute(request.into_request(client))
        .await?;

    let response: TokenPairResponse = tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/refresh")]
async fn refresh_tokens(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    request: web::Json<RefreshTokenRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: RefreshTokenRequestBody = request.into_inner();
    request.validate()?;

    let client = extract_client_context(&req);
    let tokens = app_state
        .refresh_tokens_usecase
        .execute(request.into_request(client))
        .await?;

    let response: TokenPairResponse = tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/logout")]
async fn logout(
    req: HttpRequest,
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<LogoutRequestBody>,
) -> Result<impl Responder, ApiError> {
    let request: LogoutRequestBody = request.into_inner();
    request.validate()?;
    let user_id = UserId::new(auth_token.id);

    let client = extract_client_context(&req);
    app_state
        .logout_usecase
        .execute(user_id, request.into_request(client))
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
