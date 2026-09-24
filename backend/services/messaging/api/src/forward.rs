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
    pub error: Option<ForwardFailure>,
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
            Ok(ForwardedContent { info, content: dto.content, attachments })
        }
        ForwardSourceBody::Dm { friend_id, message_id } => {
            let dto = app_state
                .list_dm_messages_usecase
                .find_one(friend_id, message_id, user_id)
                .await?;
            if dto.call_status.is_some() || dto.invite_community_id.is_some() {
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
            Ok(ForwardedContent { info, content: dto.content, attachments })
        }
    }
}

async fn deliver(
    app_state: &web::Data<AppState>,
    req: &HttpRequest,
    target: &ForwardTargetBody,
    comment: Option<&str>,
    forwarded: &ForwardedContent,
    user_id: UserId,
) -> Result<Uuid, ForwardFailure> {
    match *target {
        ForwardTargetBody::Channel { community_id, channel_id } => {
            if let Some(comment) = comment {
                let data = SendChannelMessageRequest::new(
                    community_id,
                    channel_id,
                    comment.to_string(),
                    Vec::new(),
                    None,
                    request_context(req),
                );
                app_state
                    .send_channel_message_usecase
                    .execute(data, user_id)
                    .await
                    .map_err(failure)?;
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
            let sent = app_state
                .send_channel_message_usecase
                .execute(data, user_id)
                .await
                .map_err(failure)?;
            Ok(sent.id)
        }
        ForwardTargetBody::Dm { friend_id } => {
            if let Some(comment) = comment {
                let data = SendDmMessageRequest::new(
                    friend_id,
                    comment.to_string(),
                    Vec::new(),
                    None,
                    request_context(req),
                );
                app_state
                    .send_dm_message_usecase
                    .execute(data, user_id)
                    .await
                    .map_err(failure)?;
            }
            let data = SendDmMessageRequest::new(
                friend_id,
                String::new(),
                Vec::new(),
                None,
                request_context(req),
            )
            .with_forward(forwarded.clone());
            let sent = app_state
                .send_dm_message_usecase
                .execute(data, user_id)
                .await
                .map_err(failure)?;
            Ok(sent.id)
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
        let outcome = deliver(&app_state, &req, &target, comment.as_deref(), &forwarded, user_id).await;
        results.push(match outcome {
            Ok(id) => ForwardResult { target, message_id: Some(id), error: None },
            Err(error) => ForwardResult { target, message_id: None, error: Some(error) },
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
