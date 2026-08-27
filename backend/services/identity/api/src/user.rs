use actix_web::{HttpResponse, Responder, get, patch, web};
use domain::user::{id::UserId, locale::Locale};
use serde::{Deserialize, Serialize};
use shared::middleware::auth::RateLimitedAuthToken;
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
