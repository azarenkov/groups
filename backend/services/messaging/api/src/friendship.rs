use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, web};
use messaging::dto::requests::friendship::{
    accept_friend_request::AcceptFriendRequestRequest, list_friends::ListFriendsRequest,
    remove_friendship::RemoveFriendshipRequest,
    send_friend_request::SendFriendRequestRequest,
};
use domain::user::id::UserId;
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;
use api_common::{context::request_context};
use api_common::dto::{responses::ApiResponse};
use crate::dto::{requests::friendship::{ListFriendsQuery, SendFriendRequestBody}, responses::friendship::{ FriendIdsResponse, FriendshipResponse, ListFriendshipsResponse, parse_status, }};
use api_common::dto::cursor::CursorQuery;
use crate::http_errors::HttpError;

#[post("/me/friends")]
async fn send_friend_request(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<SendFriendRequestBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);

    let data = SendFriendRequestRequest::new(body.username, request_context(&req));

    app_state
        .send_friend_request_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/me/friends/{friend_id}/accept")]
async fn accept_friend_request(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let sender_id = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = AcceptFriendRequestRequest::new(sender_id, request_context(&req));

    app_state
        .accept_friend_request_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/me/friends/{friend_id}")]
async fn remove_friendship(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let other_id = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = RemoveFriendshipRequest::new(other_id, request_context(&req));

    app_state
        .remove_friendship_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[get("/me/friends")]
async fn list_friends(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<ListFriendsQuery>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let status = match query.status.as_deref() {
        Some(s) => Some(parse_status(s).ok_or_else(|| {
            HttpError::new(
                "Unknown status".to_string(),
                actix_web::http::StatusCode::BAD_REQUEST,
                "INVALID_STATUS",
            )
        })?),
        None => None,
    };

    let paging = CursorQuery {
        cursor: query.cursor.clone(),
        limit: query.limit,
    };
    let (cursor, limit) = paging.keyset().map_err(|_| invalid_cursor())?;
    if let Some(c) = &cursor
        && Uuid::parse_str(&c.key).is_err()
    {
        return Err(invalid_cursor());
    }

    let data = ListFriendsRequest::new(status, cursor, limit, request_context(&req));

    let page = app_state.list_friends_usecase.execute(data, user_id).await?;

    let next_cursor = page.next_cursor.as_ref().map(|c| c.encode());
    let friends: Vec<FriendshipResponse> =
        page.items.into_iter().map(FriendshipResponse::from).collect();
    let response = ListFriendshipsResponse::new(friends, next_cursor);
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/me/friends/ids")]
async fn list_friend_ids(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let items = app_state.list_friends_usecase.statuses(user_id).await?;
    let response = FriendIdsResponse::new(
        items
            .into_iter()
            .map(|(id, status)| (id.uuid(), status))
            .collect(),
    );
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

fn invalid_cursor() -> HttpError {
    HttpError::new(
        "Invalid cursor".to_string(),
        actix_web::http::StatusCode::BAD_REQUEST,
        "INVALID_CURSOR",
    )
}
