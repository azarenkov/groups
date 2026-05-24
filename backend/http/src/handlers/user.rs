use actix_web::{HttpResponse, Responder, get, web};
use domain::user::id::UserId;
use serde::{Deserialize, Serialize};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::shared::{
    dto::responses::ApiResponse, error::ApiError, state::AppState,
};

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
}

#[get("/me")]
async fn get_me(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let profile = app_state.get_me_usecase.execute(user_id).await?;
    let response = MeResponse {
        id: profile.id,
        name: profile.name,
        email: profile.email,
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/search")]
async fn search_users(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<SearchUsersQuery>,
) -> Result<impl Responder, ApiError> {
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
