use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, patch, post, web};
use messaging::dto::requests::channel::{
    create_channel::CreateChannelRequest, delete_channel::DeleteChannelRequest,
    delete_channel_message::DeleteChannelMessageRequest,
    edit_channel_message::EditChannelMessageRequest,
    join_text_channel::JoinTextChannelRequest, join_voice_channel::JoinVoiceChannelRequest,
    list_channel_messages::ListChannelMessagesRequest,
    deafen_channel::DeafenChannelRequest,
    list_channel_participants::ListChannelParticipantsRequest, mute_channel::MuteChannelRequest,
    undeafen_channel::UndeafenChannelRequest,
    search_channel_messages::SearchChannelMessagesRequest,
    send_channel_message::SendChannelMessageRequest, unmute_channel::UnmuteChannelRequest,
};
use serde::Deserialize;
use messaging_domain::{channel::id::ChannelId, community::id::CommunityId};
use domain::user::id::UserId;
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;
use api_common::{context::request_context};
use api_common::dto::{pagination::{PaginationMeta, PaginationQuery}, responses::ApiResponse};
use crate::dto::responses::reply::{fill_replies, reply_ids};
use crate::dto::{requests::channel::{ create_channel::CreateChannelRequestBody, edit_message::EditMessageRequestBody, join_text_channel::JoinTextChannelRequestBody, join_voice_channel::JoinVoiceChannelRequestBody, mute_channel::MuteRequestBody, search_messages::SearchMessagesQuery, send_message::SendMessageRequestBody, unmute_channel::UnmuteRequestBody, }, responses::channel::{ create_channel::CreateChannelResponse, join_voice_channel::JoinVoiceChannelResponse, list_channel_participants::ListChannelParticipantsResponse, message::{ ListChannelMessagesResponse, MessageResponse, SearchChannelMessagesResponse, }, }};
use crate::http_errors::HttpError;

#[post("/{community_id}/channels")]
async fn create_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<CreateChannelRequestBody>,
) -> Result<impl Responder, HttpError> {
    let community_id = path.into_inner();
    let body: CreateChannelRequestBody = body.into_inner();
    body.validate()?;

    let user_id = UserId::new(auth_token.id);

    let data = CreateChannelRequest::new(
        community_id,
        body.name,
        body.kind.into(),
        body.visibility.into(),
        body.password,
        request_context(&req),
    );

    let channel = app_state
        .create_channel_usecase
        .execute(data, user_id)
        .await?;

    let response = CreateChannelResponse::from(channel);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[post("/{community_id}/channels/{channel_id}/join_voice")]
async fn join_voice_channel(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<JoinVoiceChannelRequestBody>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let body = body.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = JoinVoiceChannelRequest::new(channel_id, community_id, body.password);

    let response: JoinVoiceChannelResponse = app_state
        .join_voice_channel_usecase
        .execute(data, user_id)
        .await?
        .into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[post("/{community_id}/channels/{channel_id}/mute")]
async fn mute_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<MuteRequestBody>,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id) = path.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = MuteChannelRequest {
        channel_id,
        community_id,
        target_user_id: body.target_user_id,
        ctx: request_context(&req),
    };

    app_state
        .mute_channel_usecase
        .execute(data, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{community_id}/channels/{channel_id}/disconnect")]
async fn disconnect_voice_member(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<MuteRequestBody>,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    app_state
        .disconnect_voice_member_usecase
        .execute(
            CommunityId::new(community_id),
            ChannelId::new(channel_id),
            UserId::new(body.target_user_id),
            UserId::new(auth_token.id),
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{community_id}/channels/{channel_id}/unmute")]
async fn unmute_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UnmuteRequestBody>,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id) = path.into_inner();

    let user_id = UserId::new(auth_token.id);

    let data = UnmuteChannelRequest {
        channel_id,
        community_id,
        target_user_id: body.target_user_id,
        ctx: request_context(&req),
    };

    app_state
        .unmute_channel_usecase
        .execute(data, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{community_id}/channels/{channel_id}/deafen")]
async fn deafen_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<MuteRequestBody>,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    let data = DeafenChannelRequest {
        channel_id,
        community_id,
        target_user_id: body.target_user_id,
        ctx: request_context(&req),
    };
    app_state.deafen_channel_usecase.execute(data, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{community_id}/channels/{channel_id}/undeafen")]
async fn undeafen_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<MuteRequestBody>,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    let data = UndeafenChannelRequest {
        channel_id,
        community_id,
        target_user_id: body.target_user_id,
        ctx: request_context(&req),
    };
    app_state.undeafen_channel_usecase.execute(data, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{community_id}/channels/{channel_id}/participants")]
async fn list_channel_participants(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, HttpError> {
    query.validate()?;

    let (community_id, channel_id) = path.into_inner();
    let pagination = api_common::pagination::parse(query.page, query.per_page)?;

    let data = ListChannelParticipantsRequest::new(
        community_id,
        channel_id,
        auth_token.id,
        pagination,
        request_context(&req),
    );

    let result = app_state
        .list_channel_participants_usecase
        .execute(data)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListChannelParticipantsResponse::new(result.items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}

#[post("/{community_id}/channels/{channel_id}/join")]
async fn join_text_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Bytes,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    let password = if body.is_empty() {
        None
    } else {
        serde_json::from_slice::<JoinTextChannelRequestBody>(&body)
            .map_err(|e| {
                HttpError::new(
                    format!("Invalid request body: {e}"),
                    actix_web::http::StatusCode::BAD_REQUEST,
                    "INVALID_BODY",
                )
            })?
            .password
    };

    let data =
        JoinTextChannelRequest::new(community_id, channel_id, password, request_context(&req));

    app_state
        .join_text_channel_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}


#[get("/{community_id}/channels/{channel_id}/messages")]
async fn list_channel_messages(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<ListMessagesQuery>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = ListChannelMessagesRequest::new(
        community_id,
        channel_id,
        query.before,
        query.after,
        query.limit.unwrap_or(0),
        request_context(&req),
    );

    let messages = app_state
        .list_channel_messages_usecase
        .execute(data, user_id)
        .await?;

    let mut items: Vec<MessageResponse> =
        messages.into_iter().map(MessageResponse::from).collect();
    let ids: Vec<Uuid> = items.iter().map(|m| m.id).collect();
    let attachments = app_state
        .list_message_attachments_usecase
        .for_messages(&ids)
        .await?;
    for (mid, link) in attachments {
        if let Some(m) = items.iter_mut().find(|m| m.id == mid) {
            m.attachments.push(link.into());
        }
    }
    fill_channel_replies(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    let response = ListChannelMessagesResponse::new(items);

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/{community_id}/channels/{channel_id}/messages/search")]
async fn search_channel_messages(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<SearchMessagesQuery>,
) -> Result<impl Responder, HttpError> {
    query.validate()?;
    let (community_id, channel_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    let pagination = api_common::pagination::parse(query.page, query.per_page)?;

    let data = SearchChannelMessagesRequest::new(
        community_id,
        channel_id,
        query.q.clone(),
        pagination,
        request_context(&req),
    );

    let result = app_state
        .search_channel_messages_usecase
        .execute(data, user_id)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let mut items: Vec<MessageResponse> =
        result.items.into_iter().map(MessageResponse::from).collect();
    let ids: Vec<Uuid> = items.iter().map(|m| m.id).collect();
    let attachments = app_state
        .list_message_attachments_usecase
        .for_messages(&ids)
        .await?;
    for (mid, link) in attachments {
        if let Some(m) = items.iter_mut().find(|m| m.id == mid) {
            m.attachments.push(link.into());
        }
    }
    fill_channel_replies(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    let response = SearchChannelMessagesResponse::new(items);

    Ok(HttpResponse::Ok().json(ApiResponse::with_pagination(response, pagination_meta)))
}

#[get("/{community_id}/channels/{channel_id}/messages/{message_id}")]
async fn get_channel_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id, message_id) = path.into_inner();
    let dto = app_state
        .list_channel_messages_usecase
        .find_one(community_id, channel_id, message_id, UserId::new(auth_token.id))
        .await?;
    let mut items = vec![MessageResponse::from(dto)];
    let attachments = app_state
        .list_message_attachments_usecase
        .for_messages(&[message_id])
        .await?;
    for (_, link) in attachments {
        items[0].attachments.push(link.into());
    }
    fill_channel_replies(&app_state, &mut items, UserId::new(auth_token.id)).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(items.remove(0))))
}

#[post("/{community_id}/channels/{channel_id}/messages/{message_id}/hide")]
async fn hide_channel_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id, message_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    app_state
        .list_channel_messages_usecase
        .find_one(community_id, channel_id, message_id, user_id)
        .await?;
    app_state
        .hide_message_usecase
        .execute(user_id, message_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn fill_channel_replies(
    app_state: &web::Data<AppState>,
    items: &mut [MessageResponse],
    viewer: UserId,
) -> Result<(), HttpError> {
    let ids = reply_ids(items);
    if ids.is_empty() {
        return Ok(());
    }
    let previews = app_state
        .list_channel_messages_usecase
        .reply_previews(&ids, viewer)
        .await?;
    fill_replies(items, previews);
    Ok(())
}

#[post("/{community_id}/channels/{channel_id}/messages")]
async fn send_channel_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<SendMessageRequestBody>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let body = body.into_inner();
    body.validate()?;

    let user_id = UserId::new(auth_token.id);

    let data = SendChannelMessageRequest::new(
        community_id,
        channel_id,
        body.content,
        body.attachment_ids,
        body.reply_to_message_id,
        request_context(&req),
    );

    let response = app_state
        .send_channel_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(MessageResponse::from(response))))
}

#[patch("/{community_id}/channels/{channel_id}/messages/{message_id}")]
async fn edit_channel_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
    body: web::Json<EditMessageRequestBody>,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id, message_id) = path.into_inner();
    let body = body.into_inner();
    body.validate()?;

    let user_id = UserId::new(auth_token.id);

    let data = EditChannelMessageRequest::new(
        community_id,
        channel_id,
        message_id,
        body.content,
        body.add_attachment_ids,
        body.remove_attachment_ids,
        request_context(&req),
    );

    app_state
        .edit_channel_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{community_id}/channels/{channel_id}/messages/{message_id}")]
async fn delete_channel_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id, message_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let data = DeleteChannelMessageRequest::new(
        community_id,
        channel_id,
        message_id,
        request_context(&req),
    );

    app_state
        .delete_channel_message_usecase
        .execute(data, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{community_id}/channels/{channel_id}")]
async fn delete_channel(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let channel_id = ChannelId::new(channel_id);
    let community_id = CommunityId::new(community_id);
    let owner_id = UserId::new(auth_token.id);

    let data = DeleteChannelRequest {
        community_id,
        channel_id,
        deleted_by: owner_id,
        ctx: request_context(&req),
    };

    app_state.delete_channel_usecase.execute(data).await?;

    Ok(HttpResponse::NoContent().finish())
}
