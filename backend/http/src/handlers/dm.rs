use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, web};
use application::dto::requests::dm::{
    delete_dm_message::DeleteDmMessageRequest, edit_dm_message::EditDmMessageRequest,
    list_dm_messages::ListDmMessagesRequest, search_dm_messages::SearchDmMessagesRequest,
    send_dm_message::SendDmMessageRequest,
};
use domain::{shared::pagination::Pagination, user::id::UserId};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::shared::{
    context::request_context,
    dto::{
        pagination::PaginationMeta,
        requests::dm::{
            EditDmMessageBody, ListDmMessagesQuery, SearchDmMessagesQuery, SendDmMessageBody,
        },
        responses::{
            ApiResponse,
            dm::{DmMessageResponse, ListDmMessagesResponse, SearchDmMessagesResponse},
        },
    },
    error::ApiError,
    state::AppState,
};

#[post("/me/dms/{friend_id}/messages")]
async fn send_dm_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<SendDmMessageBody>,
) -> Result<impl Responder, ApiError> {
    let friend_id = path.into_inner();
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);

    let data = SendDmMessageRequest::new(friend_id, body.content, request_context(&req));

    let response = app_state
        .send_dm_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(DmMessageResponse::from(response))))
}

#[get("/me/dms/{friend_id}/messages")]
async fn list_dm_messages(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<ListDmMessagesQuery>,
) -> Result<impl Responder, ApiError> {
    let friend_id = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = ListDmMessagesRequest::new(
        friend_id,
        query.before,
        query.after,
        query.limit.unwrap_or(0),
        request_context(&req),
    );

    let messages = app_state
        .list_dm_messages_usecase
        .execute(data, user_id)
        .await?;

    let response =
        ListDmMessagesResponse::new(messages.into_iter().map(DmMessageResponse::from).collect());

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[patch("/me/dms/messages/{message_id}")]
async fn edit_dm_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<EditDmMessageBody>,
) -> Result<impl Responder, ApiError> {
    let message_id = path.into_inner();
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);

    let data = EditDmMessageRequest::new(message_id, body.content, request_context(&req));

    app_state
        .edit_dm_message_usecase
        .execute(data, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/me/dms/messages/{message_id}")]
async fn delete_dm_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let message_id = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = DeleteDmMessageRequest::new(message_id, request_context(&req));

    app_state
        .delete_dm_message_usecase
        .execute(data, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/me/dms/{friend_id}/messages/search")]
async fn search_dm_messages(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<SearchDmMessagesQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;
    let friend_id = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    let pagination = Pagination::new(query.page, query.per_page)?;

    let data = SearchDmMessagesRequest::new(
        friend_id,
        query.q.clone(),
        pagination,
        request_context(&req),
    );

    let result = app_state
        .search_dm_messages_usecase
        .execute(data, user_id)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = SearchDmMessagesResponse::new(
        result
            .items
            .into_iter()
            .map(DmMessageResponse::from)
            .collect(),
    );

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}
