use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use identity::dto::requests::auth::issue_verification_code::IssueVerificationCodeRequest;
use identity::usecases::auth::captcha::login_keys;
use identity::usecases::auth::google_login::GoogleLoginOutcome;
use identity::usecases::auth::login::LoginError;
use domain::user::id::UserId;
use identity_domain::verification::purpose::VerificationPurpose;
use api_common::middleware::auth::AuthToken;
use validator::Validate;

use crate::state::AppState;
use api_common::{client::extract_client_context, context::request_context};
use api_common::dto::{responses::ApiResponse};
use crate::dto::{requests::auth::{ confirm_registration::ConfirmRegistrationRequestBody, google_complete::GoogleCompleteRequestBody, google_login::GoogleLoginRequestBody, login::LoginUserRequestBody, logout::LogoutRequestBody, refresh_token::RefreshTokenRequestBody, registration::RegisterUserRequestBody, request_verification_code::RequestVerificationCodeRequestBody, }, responses::auth::{captcha::CaptchaChallengeResponse, google::GoogleAuthResponse, token_pair::TokenPairResponse}};
use crate::http_errors::HttpError;

#[get("/challenge")]
async fn captcha_challenge(app_state: web::Data<AppState>) -> impl Responder {
    let response: CaptchaChallengeResponse = app_state.captcha_gate.challenge().into();
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(ApiResponse::success(response))
}

#[post("/register")]
async fn register_user(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<RegisterUserRequestBody>,
) -> Result<impl Responder, HttpError> {
    let body: RegisterUserRequestBody = body.into_inner();
    body.validate()?;
    app_state.captcha_gate.require(body.altcha.as_deref()).await?;

    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
    let ctx = request_context(&req);
    let registered = app_state
        .register_user_usecase
        .execute(body.into_request(client, ctx))
        .await?;

    let response: TokenPairResponse = registered.tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/request_verification_code")]
async fn request_verification_code(
    req: HttpRequest,
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<RequestVerificationCodeRequestBody>,
) -> Result<impl Responder, HttpError> {
    let body: RequestVerificationCodeRequestBody = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);

    let data = IssueVerificationCodeRequest {
        purpose: VerificationPurpose::Registration,
        channel: body.channel.into(),
        ctx,
    };

    app_state
        .issue_verification_code_usecase
        .execute(user_id, data)
        .await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/confirm_registration")]
async fn confirm_registration(
    req: HttpRequest,
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<ConfirmRegistrationRequestBody>,
) -> Result<impl Responder, HttpError> {
    let request: ConfirmRegistrationRequestBody = request.into_inner();
    request.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);

    app_state
        .confirm_registration_usecase
        .execute(user_id, request.into_request(ctx))
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/login")]
async fn login_user(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    request: web::Json<LoginUserRequestBody>,
) -> Result<impl Responder, HttpError> {
    let request: LoginUserRequestBody = request.into_inner();
    request.validate()?;

    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
    let keys = login_keys(client.ip.as_deref(), &request.email);
    app_state
        .captcha_gate
        .guard_login(&keys, request.altcha.as_deref())
        .await?;
    let ctx = request_context(&req);
    let tokens = match app_state
        .login_user_usecase
        .execute(request.into_request(client, ctx))
        .await
    {
        Ok(tokens) => {
            app_state.captcha_gate.login_succeeded(&keys).await;
            tokens
        }
        Err(LoginError::InvalidCredentials) => {
            app_state.captcha_gate.login_failed(&keys).await;
            return Err(LoginError::InvalidCredentials.into());
        }
        Err(e) => return Err(e.into()),
    };

    let response: TokenPairResponse = tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/google")]
async fn google_login(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    request: web::Json<GoogleLoginRequestBody>,
) -> Result<impl Responder, HttpError> {
    let request: GoogleLoginRequestBody = request.into_inner();
    request.validate()?;

    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
    let ctx = request_context(&req);
    let outcome = app_state
        .google_login_usecase
        .execute(request.into_request(client, ctx))
        .await?;

    let response = match outcome {
        GoogleLoginOutcome::Authenticated(tokens) => {
            GoogleAuthResponse::authenticated(tokens.into())
        }
        GoogleLoginOutcome::NeedsUsername => GoogleAuthResponse::needs_username(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/google/complete")]
async fn google_complete(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    request: web::Json<GoogleCompleteRequestBody>,
) -> Result<impl Responder, HttpError> {
    let request: GoogleCompleteRequestBody = request.into_inner();
    request.validate()?;

    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
    let ctx = request_context(&req);
    let tokens = app_state
        .google_login_usecase
        .complete(request.into_request(client, ctx))
        .await?;

    let response: TokenPairResponse = tokens.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/refresh")]
async fn refresh_tokens(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    request: web::Json<RefreshTokenRequestBody>,
) -> Result<impl Responder, HttpError> {
    let request: RefreshTokenRequestBody = request.into_inner();
    request.validate()?;

    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
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
) -> Result<impl Responder, HttpError> {
    let request: LogoutRequestBody = request.into_inner();
    request.validate()?;
    let user_id = UserId::new(auth_token.id);

    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
    let ctx = request_context(&req);
    app_state
        .logout_usecase
        .execute(user_id, request.into_request(client, ctx))
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
