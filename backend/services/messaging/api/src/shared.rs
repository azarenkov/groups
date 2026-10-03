use actix_web::{HttpResponse, Responder, get, web};
use api_common::{
    dto::responses::{ApiResponse, attachment::AttachmentResponse},
    middleware::auth::RateLimitedAuthToken,
};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging::usecases::shared::list_shared_content::{SharedPage, SharedQuery};
use messaging_domain::{
    channel::id::ChannelId,
    community::id::CommunityId,
    shared::{SharedContent, SharedItem, SharedItemKey},
};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SharedParams {
    kind: String,
    q: Option<String>,
    before: Option<String>,
    limit: Option<i64>,
}

impl From<SharedParams> for SharedQuery {
    fn from(params: SharedParams) -> Self {
        Self {
            kind: params.kind,
            query: params.q,
            before: params.before,
            limit: params.limit,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedItemResponse {
    message_id: Uuid,
    author_id: Uuid,
    author_name: String,
    created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachment: Option<AttachmentResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<i64>,
}

impl From<SharedItem> for SharedItemResponse {
    fn from(item: SharedItem) -> Self {
        let position = match item.cursor.item {
            SharedItemKey::Link(ordinal) => Some(ordinal),
            SharedItemKey::Attachment(_) => None,
        };
        let (attachment, url) = match item.content {
            SharedContent::Attachment(link) => (Some(AttachmentResponse::from(link)), None),
            SharedContent::Link(url) => (None, Some(url)),
        };
        Self {
            message_id: item.message_id,
            author_id: item.author_id.uuid(),
            author_name: item.author_name,
            created_at: item.created_at,
            attachment,
            url,
            position,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedPageResponse {
    items: Vec<SharedItemResponse>,
    next_cursor: Option<String>,
}

impl From<SharedPage> for SharedPageResponse {
    fn from(page: SharedPage) -> Self {
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

#[get("/{community_id}/channels/{channel_id}/shared")]
async fn list_channel_shared(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    params: web::Query<SharedParams>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id) = path.into_inner();
    let page = app_state
        .list_shared_content_usecase
        .in_channel(
            UserId::new(auth_token.id),
            CommunityId::new(community_id),
            ChannelId::new(channel_id),
            params.into_inner().into(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(SharedPageResponse::from(page))))
}

#[get("/me/dms/{friend_id}/shared")]
async fn list_dm_shared(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    params: web::Query<SharedParams>,
) -> Result<impl Responder, HttpError> {
    let page = app_state
        .list_shared_content_usecase
        .in_dm(
            UserId::new(auth_token.id),
            UserId::new(path.into_inner()),
            params.into_inner().into(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(SharedPageResponse::from(page))))
}

pub fn channel_shared_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_channel_shared);
}

pub fn dm_shared_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_dm_shared);
}
