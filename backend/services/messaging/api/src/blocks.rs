use actix_web::{HttpResponse, Responder, delete, get, post, web};
use api_common::{dto::cursor::CursorQuery, dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::friendship::repository::FriendshipRepository;
use serde::Serialize;
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockedUserResponse {
    user_id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListBlocksResponse {
    blocks: Vec<BlockedUserResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct BlockIdsResponse {
    ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockRelationResponse {
    blocked: bool,
    blocked_by: bool,
}

#[get("/me/blocks")]
async fn list_blocks(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<CursorQuery>,
) -> Result<impl Responder, HttpError> {
    let (cursor, limit) = query.keyset().map_err(|_| invalid_cursor())?;
    if let Some(c) = &cursor
        && Uuid::parse_str(&c.key).is_err()
    {
        return Err(invalid_cursor());
    }
    let page = app_state
        .list_blocked_users_usecase
        .execute(UserId::new(auth_token.id), cursor, limit)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ListBlocksResponse {
        next_cursor: page.next_cursor.as_ref().map(|c| c.encode()),
        blocks: page
            .items
            .into_iter()
            .map(|b| BlockedUserResponse {
                user_id: b.user_id.uuid(),
                name: b.name,
                created_at: b.created_at,
            })
            .collect(),
    })))
}

#[get("/me/blocks/ids")]
async fn list_block_ids(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let ids = app_state
        .list_blocked_users_usecase
        .ids(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(BlockIdsResponse {
        ids: ids.into_iter().map(|id| id.uuid()).collect(),
    })))
}

fn invalid_cursor() -> HttpError {
    HttpError::new(
        "Invalid cursor".to_string(),
        actix_web::http::StatusCode::BAD_REQUEST,
        "INVALID_CURSOR",
    )
}

#[get("/{user_id}/block")]
async fn block_relation(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let relation = app_state
        .friendship_repository
        .block_relation(&UserId::new(auth_token.id), &UserId::new(path.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(BlockRelationResponse {
        blocked: relation.blocking,
        blocked_by: relation.blocked_by,
    })))
}

#[post("/{user_id}/block")]
async fn block_user(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let occurred_at = app_state
        .block_user_usecase
        .execute(UserId::new(auth_token.id), UserId::new(path.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(BlockChangeResponse { occurred_at })))
}

#[delete("/{user_id}/block")]
async fn unblock_user(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let occurred_at = app_state
        .unblock_user_usecase
        .execute(UserId::new(auth_token.id), UserId::new(path.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(BlockChangeResponse { occurred_at })))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockChangeResponse {
    occurred_at: DateTime<Utc>,
}

pub fn block_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_blocks)
        .service(list_block_ids)
        .service(block_relation)
        .service(block_user)
        .service(unblock_user);
}
