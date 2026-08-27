use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, web};
use messaging::dto::requests::dm::{
    delete_dm_message::DeleteDmMessageRequest, edit_dm_message::EditDmMessageRequest,
    list_dm_messages::ListDmMessagesRequest, search_dm_messages::SearchDmMessagesRequest,
    send_dm_message::SendDmMessageRequest,
};
use messaging_domain::{dm_message::id::DmMessageId, server::id::ServerId};
use domain::{shared::pagination::Pagination, user::id::UserId};
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;
use api_common::{context::request_context};
use api_common::dto::{pagination::PaginationMeta, responses::ApiResponse};
use crate::dto::{requests::dm::{ EditDmMessageBody, ListDmMessagesQuery, SearchDmMessagesQuery, SendDmMessageBody, SendServerInviteBody, }, responses::dm::{ AcceptServerInviteResponse, DmMessageResponse, ListDmMessagesResponse, SearchDmMessagesResponse, }};
use crate::http_errors::HttpError;

#[post("/me/dms/{friend_id}/messages")]
async fn send_dm_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<SendDmMessageBody>,
) -> Result<impl Responder, HttpError> {
    let friend_id = path.into_inner();
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);

    let data = SendDmMessageRequest::new(
        friend_id,
        body.content,
        body.attachment_ids,
        request_context(&req),
    );

    let response = app_state
        .send_dm_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(DmMessageResponse::from(response))))
}

#[post("/me/dms/{friend_id}/server-invite")]
async fn send_server_invite(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<SendServerInviteBody>,
) -> Result<impl Responder, HttpError> {
    let friend_id = UserId::new(path.into_inner());
    let server_id = ServerId::new(body.into_inner().server_id);
    let sender_id = UserId::new(auth_token.id);

    let response = app_state
        .send_server_invite_usecase
        .execute(server_id, friend_id, sender_id, request_context(&req))
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(DmMessageResponse::from(response))))
}

#[post("/me/dms/messages/{message_id}/accept-invite")]
async fn accept_server_invite(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let message_id = DmMessageId::new(path.into_inner());
    let requester_id = UserId::new(auth_token.id);

    let server_id = app_state
        .accept_server_invite_usecase
        .execute(message_id, requester_id, request_context(&req))
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(AcceptServerInviteResponse::new(server_id))))
}

#[post("/me/dms/messages/{message_id}/decline-invite")]
async fn decline_server_invite(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let message_id = DmMessageId::new(path.into_inner());
    let requester_id = UserId::new(auth_token.id);

    app_state
        .decline_server_invite_usecase
        .execute(message_id, requester_id, request_context(&req))
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DmPreviewItem {
    message_id: Uuid,
    peer_id: Uuid,
    sender_id: Uuid,
    content: String,
    call_status: Option<String>,
    invite_server_id: Option<Uuid>,
    has_attachments: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize)]
struct ListDmPreviewsResponse {
    previews: Vec<DmPreviewItem>,
}

#[get("/me/dms/previews")]
async fn list_dm_previews(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let previews = app_state
        .list_dm_previews_usecase
        .execute(user_id)
        .await?;
    let response = ListDmPreviewsResponse {
        previews: previews
            .into_iter()
            .map(|p| DmPreviewItem {
                message_id: p.message_id,
                peer_id: p.peer_id,
                sender_id: p.sender_id,
                content: p.content,
                call_status: p.call_status,
                invite_server_id: p.invite_server_id,
                has_attachments: p.has_attachments,
                created_at: p.created_at,
            })
            .collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/me/dms/{friend_id}/messages")]
async fn list_dm_messages(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<ListDmMessagesQuery>,
) -> Result<impl Responder, HttpError> {
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

    let mut items: Vec<DmMessageResponse> =
        messages.into_iter().map(DmMessageResponse::from).collect();
    let ids: Vec<Uuid> = items.iter().map(|m| m.id).collect();
    let attachments = app_state
        .list_message_attachments_usecase
        .for_dm_messages(&ids)
        .await?;
    for (mid, link) in attachments {
        if let Some(m) = items.iter_mut().find(|m| m.id == mid) {
            m.attachments.push(link.into());
        }
    }
    let response = ListDmMessagesResponse::new(items);

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[patch("/me/dms/messages/{message_id}")]
async fn edit_dm_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<EditDmMessageBody>,
) -> Result<impl Responder, HttpError> {
    let message_id = path.into_inner();
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);

    let data = EditDmMessageRequest::new(
        message_id,
        body.content,
        body.add_attachment_ids,
        body.remove_attachment_ids,
        request_context(&req),
    );

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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
    let mut items: Vec<DmMessageResponse> = result
        .items
        .into_iter()
        .map(DmMessageResponse::from)
        .collect();
    let ids: Vec<Uuid> = items.iter().map(|m| m.id).collect();
    let attachments = app_state
        .list_message_attachments_usecase
        .for_dm_messages(&ids)
        .await?;
    for (mid, link) in attachments {
        if let Some(m) = items.iter_mut().find(|m| m.id == mid) {
            m.attachments.push(link.into());
        }
    }
    let response = SearchDmMessagesResponse::new(items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}
