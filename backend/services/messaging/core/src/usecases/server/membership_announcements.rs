use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, message::Message, message::system::SystemMessageKind, server::unit_of_work::ServerTransaction};
use ports::repository::RepositoryError;

pub async fn announce_membership(
    tx: &mut dyn ServerTransaction,
    subject: UserId,
    kind: SystemMessageKind,
    now: DateTime<Utc>,
) -> Result<Vec<MessagingEvent>, RepositoryError> {
    let Some(channel_id) = tx.find_default_channel_id().await? else {
        return Ok(Vec::new());
    };
    let recipients = tx.list_text_channel_member_ids(&channel_id).await?;
    let mut message = Message::system_notice(channel_id, subject, kind, now);
    let events = message
        .pull_events()
        .into_iter()
        .map(|event| MessagingEvent::from(event.with_recipients(&recipients)))
        .collect();
    tx.insert_system_message(&message).await?;
    Ok(events)
}
