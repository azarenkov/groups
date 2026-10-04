use actix_web::{HttpResponse, Responder, delete, get, put, web};
use api_common::{dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::pin::{ConversationPins, PinnedMessage};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PinnedMessageResponse {
    message_id: Uuid,
    author_id: Uuid,
    author_name: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sticker_emoji: Option<String>,
    has_attachments: bool,
    created_at: DateTime<Utc>,
    pinned_by: Uuid,
    pinned_at: DateTime<Utc>,
}

impl From<PinnedMessage> for PinnedMessageResponse {
    fn from(pin: PinnedMessage) -> Self {
        Self {
            message_id: pin.message_id,
            author_id: pin.author_id.uuid(),
            author_name: pin.author_name,
            content: pin.content,
            sticker_emoji: pin.sticker_emoji,
            has_attachments: pin.has_attachments,
            created_at: pin.created_at,
            pinned_by: pin.pinned_by.uuid(),
            pinned_at: pin.pinned_at,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct PinnedMessagesResponse {
    items: Vec<PinnedMessageResponse>,
}

impl From<Vec<PinnedMessage>> for PinnedMessagesResponse {
    fn from(pins: Vec<PinnedMessage>) -> Self {
        Self {
            items: pins.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct ConversationPinsResponse {
    dms: Vec<Uuid>,
    communities: Vec<Uuid>,
}

impl From<ConversationPins> for ConversationPinsResponse {
    fn from(pins: ConversationPins) -> Self {
        Self {
            dms: pins.dms,
            communities: pins.communities,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct ConversationPinsRequest {
    ids: Vec<Uuid>,
}

#[get("/{community_id}/channels/{channel_id}/pins")]
async fn list_channel_pins(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let pins = app_state
        .message_pins_usecase
        .list_channel(community_id, channel_id, UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(PinnedMessagesResponse::from(pins))))
}

async fn set_channel_pin(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: (Uuid, Uuid, Uuid),
    pinned: bool,
) -> Result<HttpResponse, HttpError> {
    let (community_id, channel_id, message_id) = path;
    let actor = UserId::new(auth_token.id);
    let message = app_state
        .list_channel_messages_usecase
        .find_one(community_id, channel_id, message_id, actor)
        .await?;
    app_state
        .message_pins_usecase
        .set_in_channel(community_id, &message, actor, pinned)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[put("/{community_id}/channels/{channel_id}/messages/{message_id}/pin")]
async fn pin_channel_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    set_channel_pin(auth_token, app_state, path.into_inner(), true).await
}

#[delete("/{community_id}/channels/{channel_id}/messages/{message_id}/pin")]
async fn unpin_channel_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    set_channel_pin(auth_token, app_state, path.into_inner(), false).await
}

#[get("/me/dms/{friend_id}/pins")]
async fn list_dm_pins(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let pins = app_state
        .message_pins_usecase
        .list_dm(UserId::new(auth_token.id), UserId::new(path.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(PinnedMessagesResponse::from(pins))))
}

async fn set_dm_pin(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: (Uuid, Uuid),
    pinned: bool,
) -> Result<HttpResponse, HttpError> {
    let (friend_id, message_id) = path;
    let actor = UserId::new(auth_token.id);
    let message = app_state
        .list_dm_messages_usecase
        .find_one(friend_id, message_id, actor)
        .await?;
    app_state
        .message_pins_usecase
        .set_in_dm(actor, UserId::new(friend_id), &message, pinned)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[put("/me/dms/{friend_id}/messages/{message_id}/pin")]
async fn pin_dm_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    set_dm_pin(auth_token, app_state, path.into_inner(), true).await
}

#[delete("/me/dms/{friend_id}/messages/{message_id}/pin")]
async fn unpin_dm_message(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    set_dm_pin(auth_token, app_state, path.into_inner(), false).await
}

#[get("/me/pins")]
async fn list_conversation_pins(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let pins = app_state
        .conversation_pins_usecase
        .list(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ConversationPinsResponse::from(pins))))
}

#[put("/me/pins/{kind}")]
async fn replace_conversation_pins(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ConversationPinsRequest>,
) -> Result<impl Responder, HttpError> {
    let pins = app_state
        .conversation_pins_usecase
        .replace(UserId::new(auth_token.id), &path.into_inner(), body.into_inner().ids)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ConversationPinsResponse::from(pins))))
}

pub fn channel_pin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_channel_pins)
        .service(pin_channel_message)
        .service(unpin_channel_message);
}

pub fn user_pin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_dm_pins)
        .service(pin_dm_message)
        .service(unpin_dm_message)
        .service(list_conversation_pins)
        .service(replace_conversation_pins);
}
