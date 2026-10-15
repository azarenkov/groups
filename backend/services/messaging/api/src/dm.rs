use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, web};
use messaging::dto::requests::dm::{
    delete_dm_message::DeleteDmMessageRequest, edit_dm_message::EditDmMessageRequest,
    list_dm_messages::ListDmMessagesRequest, search_dm_messages::SearchDmMessagesRequest,
    send_dm_message::SendDmMessageRequest,
};
use domain::user::id::UserId;
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;
use api_common::{context::request_context};
use api_common::dto::{pagination::PaginationMeta, responses::ApiResponse};
use crate::dto::responses::reply::{fill_replies, reply_ids};
use crate::dto::{requests::dm::{ EditDmMessageBody, ListDmMessagesQuery, SearchDmMessagesQuery, SendDmMessageBody, }, responses::dm::{ DmMessageResponse, ListDmMessagesResponse, SearchDmMessagesResponse, }};
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
        body.reply_to_message_id,
        request_context(&req),
    )
    .with_sticker(body.sticker_id);

    let response = app_state
        .send_dm_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(DmMessageResponse::from(response))))
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DmPreviewItem {
    message_id: Uuid,
    peer_id: Uuid,
    peer_name: String,
    sender_id: Uuid,
    content: String,
    call_status: Option<String>,
    has_attachments: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_duration_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    video_note_duration_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sticker_emoji: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ListDmPreviewsResponse {
    previews: Vec<DmPreviewItem>,
    next_cursor: Option<String>,
}

fn invalid_cursor() -> HttpError {
    HttpError::new(
        "Invalid cursor".to_string(),
        actix_web::http::StatusCode::BAD_REQUEST,
        "INVALID_CURSOR",
    )
}

#[get("/me/dms/previews")]
async fn list_dm_previews(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<api_common::dto::cursor::CursorQuery>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let (cursor, limit) = query.keyset().map_err(|_| invalid_cursor())?;
    if let Some(c) = &cursor
        && Uuid::parse_str(&c.key).is_err()
    {
        return Err(invalid_cursor());
    }
    let page = app_state
        .list_dm_previews_usecase
        .execute(user_id, cursor, limit)
        .await?;
    let response = ListDmPreviewsResponse {
        next_cursor: page.next_cursor.as_ref().map(|c| c.encode()),
        previews: page
            .items
            .into_iter()
            .map(|p| DmPreviewItem {
                message_id: p.message_id,
                peer_id: p.peer_id,
                peer_name: p.peer_name,
                sender_id: p.sender_id,
                content: p.content,
                call_status: p.call_status,
                has_attachments: p.has_attachments,
                voice_duration_ms: p.voice_duration_ms,
                video_note_duration_ms: p.video_note_duration_ms,
                sticker_emoji: p.sticker_emoji,
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

    let partition = infrastructure_common::messaging::partitions::conversation_key(user_id.uuid(), friend_id);
    let seq = app_state.partition_sequences.current(partition).await.map_err(HttpError::from_error)?;
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
    fill_dm_replies(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    crate::voice::fill_dm_listens(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    let response = ListDmMessagesResponse::new(
        items,
        infrastructure_common::messaging::partitions::partition_id(messaging_infrastructure::database::postgres::SCHEMA, partition),
        seq,
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/me/dms/{friend_id}/messages/{message_id}")]
async fn get_dm_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (friend_id, message_id) = path.into_inner();
    let dto = app_state
        .list_dm_messages_usecase
        .find_one(friend_id, message_id, UserId::new(auth_token.id))
        .await?;
    let mut items = vec![DmMessageResponse::from(dto)];
    let attachments = app_state
        .list_message_attachments_usecase
        .for_dm_messages(&[message_id])
        .await?;
    for (_, link) in attachments {
        items[0].attachments.push(link.into());
    }
    fill_dm_replies(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    crate::voice::fill_dm_listens(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(items.remove(0))))
}

#[post("/me/dms/{friend_id}/messages/{message_id}/hide")]
async fn hide_dm_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (friend_id, message_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    app_state
        .list_dm_messages_usecase
        .find_one(friend_id, message_id, user_id)
        .await?;
    app_state
        .hide_message_usecase
        .execute(user_id, message_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn fill_dm_replies(
    app_state: &web::Data<AppState>,
    items: &mut [DmMessageResponse],
    viewer: UserId,
) -> Result<(), HttpError> {
    let ids = reply_ids(items);
    if ids.is_empty() {
        return Ok(());
    }
    let previews = app_state.list_dm_messages_usecase.reply_previews(&ids, viewer).await?;
    fill_replies(items, previews);
    Ok(())
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
    let pagination = api_common::pagination::parse(query.page, query.per_page)?;

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
    fill_dm_replies(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    crate::voice::fill_dm_listens(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    let response = SearchDmMessagesResponse::new(items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}
