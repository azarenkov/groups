use actix_web::{HttpResponse, Responder, post, web};
use api_common::middleware::auth::RateLimitedAuthToken;
use domain::user::id::UserId;
use uuid::Uuid;

use crate::dto::responses::channel::message::MessageResponse;
use crate::dto::responses::dm::DmMessageResponse;
use crate::http_errors::HttpError;
use crate::state::AppState;

fn voice_ids<'a>(items: impl Iterator<Item = (Uuid, bool)> + 'a) -> Vec<Uuid> {
    items.filter(|(_, voice)| *voice).map(|(id, _)| id).collect()
}

pub async fn fill_channel_listens(
    app_state: &web::Data<AppState>,
    items: &mut [MessageResponse],
    viewer: UserId,
) -> Result<(), HttpError> {
    let ids = voice_ids(items.iter().map(|m| (m.id, m.attachments.iter().any(|a| a.voice.is_some()))));
    let listened = app_state.voice_listens_usecase.in_channel(&ids, viewer).await?;
    for item in items.iter_mut() {
        item.voice_listened = listened.contains(&item.id);
    }
    Ok(())
}

pub async fn fill_dm_listens(
    app_state: &web::Data<AppState>,
    items: &mut [DmMessageResponse],
    viewer: UserId,
) -> Result<(), HttpError> {
    let ids = voice_ids(items.iter().map(|m| (m.id, m.attachments.iter().any(|a| a.voice.is_some()))));
    let listened = app_state.voice_listens_usecase.in_dm(&ids, viewer).await?;
    for item in items.iter_mut() {
        item.voice_listened = listened.contains(&item.id);
    }
    Ok(())
}

#[post("/{community_id}/channels/{channel_id}/messages/{message_id}/listened")]
async fn mark_channel_voice_listened(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (community_id, channel_id, message_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    let message = app_state
        .list_channel_messages_usecase
        .find_one(community_id, channel_id, message_id, user_id)
        .await?;
    let attachments: Vec<_> = app_state
        .list_message_attachments_usecase
        .for_messages(&[message_id])
        .await?
        .into_iter()
        .map(|(_, link)| link)
        .collect();
    app_state
        .voice_listens_usecase
        .mark(message_id, message.author_id, user_id, &attachments, &[])
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/me/dms/{friend_id}/messages/{message_id}/listened")]
async fn mark_dm_voice_listened(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (friend_id, message_id) = path.into_inner();
    let user_id = UserId::new(auth_token.id);
    let message = app_state
        .list_dm_messages_usecase
        .find_one(friend_id, message_id, user_id)
        .await?;
    let attachments: Vec<_> = app_state
        .list_message_attachments_usecase
        .for_dm_messages(&[message_id])
        .await?
        .into_iter()
        .map(|(_, link)| link)
        .collect();
    app_state
        .voice_listens_usecase
        .mark(message_id, message.sender_id, user_id, &attachments, &[message.sender_id])
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
