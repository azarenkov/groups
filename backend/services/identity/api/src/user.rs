use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, web};
use api_common::context::request_context;
use identity::usecases::user::update_profile::ProfilePatch;
use identity::usecases::user::{
    change_password::ChangePasswordRequest, confirm_email_change::ConfirmEmailChangeRequest,
    reauth::Reauth, request_email_change::RequestEmailChangeRequest,
};
use validator::Validate;
use identity_domain::user::{id::UserId, locale::Locale};
use serde::{Deserialize, Serialize};
use api_common::client::extract_client_context;
use api_common::middleware::auth::{RateLimitedAuthToken, RateLimitedUnverifiedAuthToken};
use crate::dto::responses::auth::token_pair::TokenPairResponse;
use uuid::Uuid;

use crate::state::AppState;
use api_common::dto::responses::ApiResponse;
use crate::http_errors::HttpError;

#[derive(Debug, Deserialize)]
pub struct SearchUsersQuery {
    pub q: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchItem {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct UserSearchResponse {
    pub users: Vec<UserSearchItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub locale: String,
    pub email_verified: bool,
    pub password_changed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub linked_providers: Vec<String>,
    pub bio: String,
    pub show_activity: bool,
}

#[get("/me")]
async fn get_me(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let profile = app_state.get_me_usecase.execute(user_id).await?;
    let response = MeResponse {
        id: profile.id,
        name: profile.name,
        email: profile.email,
        locale: profile.locale.as_str().to_string(),
        email_verified: profile.email_verified,
        password_changed_at: profile.password_changed_at,
        linked_providers: profile.linked_providers,
        bio: profile.bio,
        show_activity: profile.show_activity,
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/search")]
async fn search_users(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<SearchUsersQuery>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);

    let items = app_state
        .search_users_usecase
        .execute(&query.q, query.limit, user_id)
        .await?;

    let response = UserSearchResponse {
        users: items
            .into_iter()
            .map(|u| UserSearchItem {
                id: u.id,
                name: u.name,
            })
            .collect(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLocaleBody {
    pub locale: String,
}

#[patch("/me/locale")]
async fn update_me_locale(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<UpdateLocaleBody>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let locale: Locale = body.locale.parse().map_err(|_| {
        HttpError::new(
            format!("invalid locale: {}", body.locale),
            actix_web::http::StatusCode::BAD_REQUEST,
            "INVALID_LOCALE",
        )
    })?;
    app_state
        .update_user_locale_usecase
        .execute(user_id, locale)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNameBody {
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNameResponse {
    pub name: String,
}

#[patch("/me/name")]
async fn update_me_name(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<UpdateNameBody>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let name = app_state
        .rename_user_usecase
        .execute(user_id, &body.name)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(UpdateNameResponse { name })))
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordBody {
    #[validate(length(min = 1, max = 128))]
    pub current_password: Option<String>,
    pub google_id_token: Option<String>,
    #[validate(length(min = 4, max = 30))]
    pub new_password: String,
}

fn reauth_from(password: Option<String>, google_id_token: Option<String>) -> Result<Reauth, HttpError> {
    match (password, google_id_token) {
        (Some(password), _) if !password.is_empty() => Ok(Reauth::Password(password)),
        (_, Some(token)) if !token.is_empty() => Ok(Reauth::GoogleIdToken(token)),
        _ => Err(HttpError::new(
            "Provide a password or a Google id token".to_string(),
            actix_web::http::StatusCode::BAD_REQUEST,
            "REAUTH_REQUIRED",
        )),
    }
}

#[patch("/me/password")]
async fn update_me_password(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<UpdatePasswordBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let client = extract_client_context(&req, app_state.trust_forwarded_headers);
    let tokens = app_state
        .change_password_usecase
        .execute(
            user_id,
            ChangePasswordRequest {
                reauth: reauth_from(body.current_password, body.google_id_token)?,
                new_password: body.new_password,
                ip: client.ip,
                user_agent: client.user_agent,
                request_id: ctx.request_id,
            },
        )
        .await?;
    let response: TokenPairResponse = tokens.into();
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEmailBody {
    #[validate(length(min = 1, max = 128))]
    pub password: Option<String>,
    pub google_id_token: Option<String>,
    #[validate(email)]
    pub new_email: String,
}

#[post("/me/email/change")]
async fn change_me_email(
    req: HttpRequest,
    auth_token: RateLimitedUnverifiedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<ChangeEmailBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    app_state
        .request_email_change_usecase
        .execute(
            user_id,
            RequestEmailChangeRequest {
                reauth: reauth_from(body.password, body.google_id_token)?,
                new_email: body.new_email,
                request_id: ctx.request_id,
            },
        )
        .await?;
    Ok(HttpResponse::Accepted().finish())
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmEmailBody {
    #[validate(length(min = 4, max = 12))]
    pub verification_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmEmailResponse {
    pub email: String,
}

#[post("/me/email/confirm")]
async fn confirm_me_email(
    req: HttpRequest,
    auth_token: RateLimitedUnverifiedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<ConfirmEmailBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let email = app_state
        .confirm_email_change_usecase
        .execute(
            user_id,
            ConfirmEmailChangeRequest {
                verification_code: body.verification_code,
                request_id: ctx.request_id,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ConfirmEmailResponse { email })))
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PasswordBody {
    #[validate(length(min = 1, max = 128))]
    pub password: Option<String>,
    pub google_id_token: Option<String>,
}

#[post("/me/disable")]
async fn disable_me(
    req: HttpRequest,
    auth_token: RateLimitedUnverifiedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<PasswordBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let reauth = reauth_from(body.password, body.google_id_token)?;
    app_state
        .disable_account_usecase
        .execute(user_id, &reauth, ctx.request_id.as_deref())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/me")]
async fn delete_me(
    req: HttpRequest,
    auth_token: RateLimitedUnverifiedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<PasswordBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let reauth = reauth_from(body.password, body.google_id_token)?;
    app_state
        .delete_account_usecase
        .execute(user_id, &reauth, ctx.request_id.as_deref())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileBody {
    #[validate(length(max = 190))]
    pub bio: Option<String>,
    pub show_activity: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileResponse {
    pub bio: String,
    pub show_activity: bool,
    pub profile_updated_at: chrono::DateTime<chrono::Utc>,
}

#[patch("/me/profile")]
async fn update_me_profile(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<UpdateProfileBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let (bio, show_activity, profile_updated_at) = app_state
        .update_profile_usecase
        .execute(
            user_id,
            ProfilePatch {
                bio: body.bio,
                show_activity: body.show_activity,
                request_id: ctx.request_id,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(UpdateProfileResponse { bio, show_activity, profile_updated_at })))
}
