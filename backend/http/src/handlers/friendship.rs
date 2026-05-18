use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, web};
use application::dto::requests::friendship::{
    accept_friend_request::AcceptFriendRequestRequest, list_friends::ListFriendsRequest,
    remove_friendship::RemoveFriendshipRequest,
    send_friend_request::SendFriendRequestRequest,
};
use domain::user::id::UserId;
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::shared::{
    context::request_context,
    dto::{
        requests::friendship::{ListFriendsQuery, SendFriendRequestBody},
        responses::{
            ApiResponse,
            friendship::{
                FriendshipResponse, ListFriendshipsResponse, parse_status,
            },
        },
    },
    error::ApiError,
    state::AppState,
};

#[post("/me/friends")]
async fn send_friend_request(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<SendFriendRequestBody>,
) -> Result<impl Responder, ApiError> {
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
) -> Result<impl Responder, ApiError> {
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
) -> Result<impl Responder, ApiError> {
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
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let status = match query.status.as_deref() {
        Some(s) => Some(parse_status(s).ok_or_else(|| {
            ApiError::new(
                "Unknown status".to_string(),
                actix_web::http::StatusCode::BAD_REQUEST,
                "INVALID_STATUS",
            )
        })?),
        None => None,
    };

    let data = ListFriendsRequest::new(status, request_context(&req));

    let result = app_state.list_friends_usecase.execute(data, user_id).await?;

    let friends: Vec<FriendshipResponse> =
        result.into_iter().map(FriendshipResponse::from).collect();
    let response = ListFriendshipsResponse::new(friends);
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}
