use actix_web::{HttpRequest, HttpResponse, Responder, http::StatusCode, post, web};
use api_common::{context::request_context, dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use domain::user::id::UserId;
use messaging::dto::requests::{
    channel::send_channel_message::SendChannelMessageRequest,
    dm::send_dm_message::SendDmMessageRequest,
    forward::ForwardedContent,
};
use messaging_domain::message::forward::{ForwardInfo, ForwardSource};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::http_errors::{HttpError, HttpMapping};
use crate::state::AppState;

pub const MAX_FORWARD_TARGETS: usize = 10;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind")]
pub enum ForwardSourceBody {
    #[serde(rename = "channel", rename_all = "camelCase")]
    Channel {
        community_id: Uuid,
        channel_id: Uuid,
        message_id: Uuid,
    },
    #[serde(rename = "dm", rename_all = "camelCase")]
    Dm { friend_id: Uuid, message_id: Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum ForwardTargetBody {
    #[serde(rename = "channel", rename_all = "camelCase")]
    Channel { community_id: Uuid, channel_id: Uuid },
    #[serde(rename = "dm", rename_all = "camelCase")]
    Dm { friend_id: Uuid },
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ForwardMessageBody {
    pub source: ForwardSourceBody,
    #[validate(length(min = 1, max = 10))]
    pub targets: Vec<ForwardTargetBody>,
    #[serde(default)]
    #[validate(length(max = 2000))]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardFailure {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardResult {
    pub target: ForwardTargetBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_message_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ForwardFailure>,
}

struct Delivery {
    message_id: Option<Uuid>,
    comment_message_id: Option<Uuid>,
    error: Option<ForwardFailure>,
}

impl Delivery {
    fn failed(comment_message_id: Option<Uuid>, error: ForwardFailure) -> Self {
        Self { message_id: None, comment_message_id, error: Some(error) }
    }

    fn sent(message_id: Uuid, comment_message_id: Option<Uuid>) -> Self {
        Self { message_id: Some(message_id), comment_message_id, error: None }
    }
}

#[derive(Debug, Serialize)]
pub struct ForwardResponse {
    pub results: Vec<ForwardResult>,
}

fn failure<E: HttpMapping>(error: E) -> ForwardFailure {
    ForwardFailure {
        code: error.to_error_code().to_string(),
        message: error.to_user_message(),
    }
}

pub fn unique_targets(targets: Vec<ForwardTargetBody>) -> Vec<ForwardTargetBody> {
    let mut out: Vec<ForwardTargetBody> = Vec::new();
    for target in targets {
        if !out.contains(&target) {
            out.push(target);
        }
    }
    out
}

fn not_forwardable() -> HttpError {
    HttpError::new(
        "This message cannot be forwarded.".to_string(),
        StatusCode::BAD_REQUEST,
        "FORWARD_INVALID",
    )
}

async fn resolve_source(
    app_state: &web::Data<AppState>,
    source: &ForwardSourceBody,
    user_id: UserId,
) -> Result<ForwardedContent, HttpError> {
    match *source {
        ForwardSourceBody::Channel { community_id, channel_id, message_id } => {
            let dto = app_state
                .list_channel_messages_usecase
                .find_one(community_id, channel_id, message_id, user_id)
                .await?;
            if dto.system.is_some() {
                return Err(not_forwardable());
            }
            let attachments = app_state
                .list_message_attachments_usecase
                .for_messages(&[message_id])
                .await?
                .into_iter()
                .map(|(_, link)| link)
                .collect();
            let info = dto.forwarded_from.clone().unwrap_or(ForwardInfo {
                message_id,
                author_id: UserId::new(dto.author_id),
                author_name: dto.author_name.clone().unwrap_or_default(),
                sent_at: dto.created_at,
                source: ForwardSource::Channel { community_id, channel_id },
            });
            Ok(ForwardedContent { info, content: dto.content, attachments, sticker: dto.sticker })
        }
        ForwardSourceBody::Dm { friend_id, message_id } => {
            let dto = app_state
                .list_dm_messages_usecase
                .find_one(friend_id, message_id, user_id)
                .await?;
            if dto.call_status.is_some() {
                return Err(not_forwardable());
            }
            let attachments = app_state
                .list_message_attachments_usecase
                .for_dm_messages(&[message_id])
                .await?
                .into_iter()
                .map(|(_, link)| link)
                .collect();
            let info = dto.forwarded_from.clone().unwrap_or(ForwardInfo {
                message_id,
                author_id: UserId::new(dto.sender_id),
                author_name: dto.sender_name.clone(),
                sent_at: dto.created_at,
                source: ForwardSource::Dm,
            });
            Ok(ForwardedContent { info, content: dto.content, attachments, sticker: dto.sticker })
        }
    }
}

async fn source_still_visible(
    app_state: &web::Data<AppState>,
    source: &ForwardSourceBody,
    user_id: UserId,
) -> Result<(), ForwardFailure> {
    if let ForwardSourceBody::Dm { friend_id, message_id } = *source {
        app_state
            .list_dm_messages_usecase
            .find_one(friend_id, message_id, user_id)
            .await
            .map_err(failure)?;
    }
    Ok(())
}

async fn deliver(
    app_state: &web::Data<AppState>,
    req: &HttpRequest,
    target: &ForwardTargetBody,
    comment: Option<&str>,
    forwarded: &ForwardedContent,
    user_id: UserId,
) -> Delivery {
    match *target {
        ForwardTargetBody::Channel { community_id, channel_id } => {
            let join = messaging::dto::requests::channel::join_text_channel::JoinTextChannelRequest::new(
                community_id,
                channel_id,
                None,
                request_context(req),
            );
            if let Err(e) = app_state.join_text_channel_usecase.execute(join, user_id).await {
                return Delivery::failed(None, failure(e));
            }
            let mut comment_message_id = None;
            if let Some(comment) = comment {
                let data = SendChannelMessageRequest::new(
                    community_id,
                    channel_id,
                    comment.to_string(),
                    Vec::new(),
                    None,
                    request_context(req),
                );
                match app_state.send_channel_message_usecase.execute(data, user_id).await {
                    Ok(sent) => comment_message_id = Some(sent.id),
                    Err(e) => return Delivery::failed(None, failure(e)),
                }
            }
            let data = SendChannelMessageRequest::new(
                community_id,
                channel_id,
                String::new(),
                Vec::new(),
                None,
                request_context(req),
            )
            .with_forward(forwarded.clone());
            match app_state.send_channel_message_usecase.execute(data, user_id).await {
                Ok(sent) => Delivery::sent(sent.id, comment_message_id),
                Err(e) => Delivery::failed(comment_message_id, failure(e)),
            }
        }
        ForwardTargetBody::Dm { friend_id } => {
            let mut comment_message_id = None;
            if let Some(comment) = comment {
                let data = SendDmMessageRequest::new(
                    friend_id,
                    comment.to_string(),
                    Vec::new(),
                    None,
                    request_context(req),
                );
                match app_state.send_dm_message_usecase.execute(data, user_id).await {
                    Ok(sent) => comment_message_id = Some(sent.id),
                    Err(e) => return Delivery::failed(None, failure(e)),
                }
            }
            let data = SendDmMessageRequest::new(
                friend_id,
                String::new(),
                Vec::new(),
                None,
                request_context(req),
            )
            .with_forward(forwarded.clone());
            match app_state.send_dm_message_usecase.execute(data, user_id).await {
                Ok(sent) => Delivery::sent(sent.id, comment_message_id),
                Err(e) => Delivery::failed(comment_message_id, failure(e)),
            }
        }
    }
}

#[post("/me/forward")]
async fn forward_message(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<ForwardMessageBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    body.validate()?;
    let user_id = UserId::new(auth_token.id);
    let forwarded = resolve_source(&app_state, &body.source, user_id).await?;
    let comment = body
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string);
    let mut results = Vec::new();
    for target in unique_targets(body.targets).into_iter().take(MAX_FORWARD_TARGETS) {
        let outcome = match source_still_visible(&app_state, &body.source, user_id).await {
            Ok(()) => deliver(&app_state, &req, &target, comment.as_deref(), &forwarded, user_id).await,
            Err(gone) => Delivery::failed(None, gone),
        };
        results.push(ForwardResult {
            target,
            message_id: outcome.message_id,
            comment_message_id: outcome.comment_message_id,
            error: outcome.error,
        });
    }
    Ok(HttpResponse::Ok().json(ApiResponse::success(ForwardResponse { results })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets_of_both_kinds() {
        let body: ForwardMessageBody = serde_json::from_value(serde_json::json!({
            "source": { "kind": "dm", "friendId": Uuid::nil(), "messageId": Uuid::nil() },
            "targets": [
                { "kind": "channel", "communityId": Uuid::nil(), "channelId": Uuid::nil() },
                { "kind": "dm", "friendId": Uuid::nil() }
            ],
            "comment": "look"
        }))
        .unwrap();
        assert_eq!(body.targets.len(), 2);
        assert!(matches!(body.source, ForwardSourceBody::Dm { .. }));
        assert!(body.validate().is_ok());
    }

    #[test]
    fn rejects_an_empty_target_list() {
        let body: ForwardMessageBody = serde_json::from_value(serde_json::json!({
            "source": { "kind": "dm", "friendId": Uuid::nil(), "messageId": Uuid::nil() },
            "targets": []
        }))
        .unwrap();
        assert!(body.validate().is_err());
    }

    #[test]
    fn delivers_to_each_target_once() {
        let a = ForwardTargetBody::Dm { friend_id: Uuid::new_v4() };
        let b = ForwardTargetBody::Dm { friend_id: Uuid::new_v4() };
        assert_eq!(unique_targets(vec![a.clone(), b.clone(), a.clone()]), vec![a, b]);
    }
}
