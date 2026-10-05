use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::pin::{
    ConversationPinKind, ConversationPinOutcome, ConversationPinRepository, ConversationPins, MAX_PINNED_CONVERSATIONS,
    MAX_PINNED_MESSAGES, MessagePinRepository,
    PinOutcome, PinScope, PinnedMessage,
};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresMessagePinRepository {
    pool: Pool<Postgres>,
}

impl PostgresMessagePinRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

fn scope_key(scope: PinScope) -> String {
    match scope {
        PinScope::Channel(channel_id) => format!("pins:channel:{channel_id}"),
        PinScope::Dm { user_low, user_high } => format!("pins:dm:{user_low}:{user_high}"),
    }
}

fn pin_event_record(
    scope: PinScope,
    message_id: Uuid,
    actor: &UserId,
    pinned: bool,
    occurred_at: DateTime<Utc>,
    recipients: &[Uuid],
) -> OutboxRecord {
    let (channel_id, participants, aggregate_type, parent_id, parent_type, sender_id, recipient_id) = match scope {
        PinScope::Channel(channel_id) => (
            Some(channel_id),
            Vec::new(),
            AggregateType::Message,
            Some(channel_id),
            Some(AggregateType::Channel),
            None,
            None,
        ),
        PinScope::Dm { user_low, user_high } => (
            None,
            vec![user_low, user_high],
            AggregateType::DmMessage,
            None,
            None,
            Some(user_low),
            Some(user_high),
        ),
    };
    let mut recipients = recipients.to_vec();
    if !recipients.contains(&actor.uuid()) {
        recipients.push(actor.uuid());
    }
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: message_id,
        aggregate_type,
        parent_id,
        parent_type,
        event_type: "message.pin_changed".to_string(),
        payload: serde_json::json!({
            "message_id": message_id,
            "sender_id": sender_id,
            "recipient_id": recipient_id,
            "channel_id": channel_id,
            "participants": participants,
            "pinned": pinned,
            "actor_id": actor.uuid(),
            "occurred_at": occurred_at,
            "recipients": recipients,
        }),
        occurred_at,
        request_id: None,
    }
}

async fn save_pin_event(
    tx: &mut Transaction<'_, Postgres>,
    scope: PinScope,
    message_id: Uuid,
    actor: &UserId,
    pinned: bool,
    occurred_at: DateTime<Utc>,
    recipients: &[Uuid],
) -> Result<(), RepositoryError> {
    let record = pin_event_record(scope, message_id, actor, pinned, occurred_at, recipients);
    save_outbox_records(tx, std::slice::from_ref(&record)).await
}

#[async_trait]
impl MessagePinRepository for PostgresMessagePinRepository {
    async fn pin(
        &self,
        scope: PinScope,
        message_id: Uuid,
        pinned_by: &UserId,
        recipients: &[Uuid],
    ) -> Result<PinOutcome, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 51))")
            .bind(scope_key(scope))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let (pinned_at, count) = match scope {
            PinScope::Channel(channel_id) => {
                let pinned_at = sqlx::query_scalar!(
                    r#"
                    INSERT INTO channel_pinned_messages (message_id, channel_id, pinned_by)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (message_id) DO NOTHING
                    RETURNING pinned_at
                    "#,
                    message_id,
                    channel_id,
                    pinned_by.uuid(),
                )
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                let count = sqlx::query_scalar!(
                    r#"SELECT COUNT(*) AS "count!" FROM channel_pinned_messages WHERE channel_id = $1"#,
                    channel_id,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                (pinned_at, count)
            }
            PinScope::Dm { user_low, user_high } => {
                let pinned_at = sqlx::query_scalar!(
                    r#"
                    INSERT INTO dm_pinned_messages (message_id, user_low, user_high, pinned_by)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (message_id) DO NOTHING
                    RETURNING pinned_at
                    "#,
                    message_id,
                    user_low,
                    user_high,
                    pinned_by.uuid(),
                )
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                let count = sqlx::query_scalar!(
                    r#"SELECT COUNT(*) AS "count!" FROM dm_pinned_messages WHERE user_low = $1 AND user_high = $2"#,
                    user_low,
                    user_high,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                (pinned_at, count)
            }
        };
        let Some(pinned_at) = pinned_at else {
            return Ok(PinOutcome::AlreadyPinned);
        };
        if count > MAX_PINNED_MESSAGES {
            return Ok(PinOutcome::LimitReached);
        }
        save_pin_event(&mut tx, scope, message_id, pinned_by, true, pinned_at, recipients).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(PinOutcome::Pinned(pinned_at))
    }

    async fn unpin(
        &self,
        scope: PinScope,
        message_id: Uuid,
        unpinned_by: &UserId,
        recipients: &[Uuid],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let removed = match scope {
            PinScope::Channel(channel_id) => sqlx::query_scalar!(
                r#"DELETE FROM channel_pinned_messages WHERE message_id = $1 AND channel_id = $2 RETURNING message_id"#,
                message_id,
                channel_id,
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
            PinScope::Dm { user_low, user_high } => sqlx::query_scalar!(
                r#"DELETE FROM dm_pinned_messages WHERE message_id = $1 AND user_low = $2 AND user_high = $3 RETURNING message_id"#,
                message_id,
                user_low,
                user_high,
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
        };
        if removed.is_none() {
            return Ok(false);
        }
        save_pin_event(&mut tx, scope, message_id, unpinned_by, false, Utc::now(), recipients).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn list(&self, scope: PinScope, viewer: &UserId) -> Result<Vec<PinnedMessage>, RepositoryError> {
        let rows = match scope {
            PinScope::Channel(channel_id) => sqlx::query_as!(
                PinnedRow,
                r#"
                SELECT p.message_id AS "message_id!", m.author_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                       (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                       m.content AS "content!", m.sticker->>'emoji' AS "sticker_emoji?",
                       EXISTS (SELECT 1 FROM message_attachments a WHERE a.message_id = m.id) AS "has_attachments!",
                       m.created_at AS "created_at!", p.pinned_by AS "pinned_by!", p.pinned_at AS "pinned_at!"
                FROM channel_pinned_messages p
                JOIN messages m ON m.id = p.message_id
                LEFT JOIN messaging_user_profiles u ON u.user_id = m.author_id
                WHERE p.channel_id = $1
                  AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $2 AND h.message_id = m.id)
                ORDER BY p.pinned_at DESC, p.message_id DESC
                LIMIT $3
                "#,
                channel_id,
                viewer.uuid(),
                MAX_PINNED_MESSAGES,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
            PinScope::Dm { user_low, user_high } => sqlx::query_as!(
                PinnedRow,
                r#"
                SELECT p.message_id AS "message_id!", m.sender_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                       (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                       m.content AS "content!", m.sticker->>'emoji' AS "sticker_emoji?",
                       EXISTS (SELECT 1 FROM dm_message_attachments a WHERE a.dm_message_id = m.id) AS "has_attachments!",
                       m.created_at AS "created_at!", p.pinned_by AS "pinned_by!", p.pinned_at AS "pinned_at!"
                FROM dm_pinned_messages p
                JOIN dm_messages m ON m.id = p.message_id
                LEFT JOIN messaging_user_profiles u ON u.user_id = m.sender_id
                WHERE p.user_low = $1 AND p.user_high = $2
                  AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $3 AND h.message_id = m.id)
                ORDER BY p.pinned_at DESC, p.message_id DESC
                LIMIT $4
                "#,
                user_low,
                user_high,
                viewer.uuid(),
                MAX_PINNED_MESSAGES,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
        };
        Ok(rows.into_iter().map(PinnedMessage::from).collect())
    }
}

struct PinnedRow {
    message_id: Uuid,
    author_id: Uuid,
    author_name: String,
    forwarded_author_id: Option<Uuid>,
    content: String,
    sticker_emoji: Option<String>,
    has_attachments: bool,
    created_at: DateTime<Utc>,
    pinned_by: Uuid,
    pinned_at: DateTime<Utc>,
}

impl From<PinnedRow> for PinnedMessage {
    fn from(r: PinnedRow) -> Self {
        Self {
            message_id: r.message_id,
            author_id: UserId::new(r.author_id),
            author_name: r.author_name,
            forwarded_author_id: r.forwarded_author_id.map(UserId::new),
            content: r.content,
            sticker_emoji: r.sticker_emoji,
            has_attachments: r.has_attachments,
            created_at: r.created_at,
            pinned_by: UserId::new(r.pinned_by),
            pinned_at: r.pinned_at,
        }
    }
}

pub struct PostgresConversationPinRepository {
    pool: Pool<Postgres>,
}

impl PostgresConversationPinRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

async fn load_conversation_pins(
    executor: impl sqlx::PgExecutor<'_>,
    user: &UserId,
) -> Result<ConversationPins, RepositoryError> {
    let rows = sqlx::query!(
        r#"
        SELECT p.kind, p.target_id FROM conversation_pins p
        WHERE p.user_id = $1
          AND (p.kind <> 'community'
               OR EXISTS (SELECT 1 FROM community_members m WHERE m.community_id = p.target_id AND m.user_id = p.user_id))
          AND (p.kind <> 'dm'
               OR NOT EXISTS (
                   SELECT 1 FROM user_blocks b
                   WHERE (b.blocker_id = p.user_id AND b.blocked_id = p.target_id)
                      OR (b.blocker_id = p.target_id AND b.blocked_id = p.user_id)
               ))
        ORDER BY p."position", p.pinned_at
        "#,
        user.uuid(),
    )
    .fetch_all(executor)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    let mut pins = ConversationPins::default();
    for row in rows {
        match ConversationPinKind::parse(&row.kind) {
            Some(ConversationPinKind::Dm) => pins.dms.push(row.target_id),
            Some(ConversationPinKind::Community) => pins.communities.push(row.target_id),
            None => {}
        }
    }
    Ok(pins)
}

#[async_trait]
impl ConversationPinRepository for PostgresConversationPinRepository {
    async fn list(&self, user: &UserId) -> Result<ConversationPins, RepositoryError> {
        load_conversation_pins(&self.pool, user).await
    }

    async fn pin(
        &self,
        user: &UserId,
        kind: ConversationPinKind,
        target: Uuid,
    ) -> Result<ConversationPinOutcome, RepositoryError> {
        let mut tx = begin_pins_tx(&self.pool, user).await?;
        sqlx::query!(
            r#"
            INSERT INTO conversation_pins (user_id, kind, target_id, "position")
            SELECT $1, $2, $3,
                   COALESCE((SELECT MIN(p."position") FROM conversation_pins p WHERE p.user_id = $1 AND p.kind = $2), 1) - 1
            WHERE $2 <> 'dm'
               OR (EXISTS (SELECT 1 FROM messaging_user_profiles u WHERE u.user_id = $3)
                   AND NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = $3))
            ON CONFLICT (user_id, kind, target_id) DO NOTHING
            "#,
            user.uuid(),
            kind.as_str(),
            target,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let pins = load_conversation_pins(&mut *tx, user).await?;
        let list = match kind {
            ConversationPinKind::Dm => &pins.dms,
            ConversationPinKind::Community => &pins.communities,
        };
        if !list.contains(&target) {
            return Ok(ConversationPinOutcome::UnknownTarget);
        }
        if list.len() > MAX_PINNED_CONVERSATIONS {
            return Ok(ConversationPinOutcome::LimitReached);
        }
        save_pins_event(&mut tx, user, &pins).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(ConversationPinOutcome::Updated(pins))
    }

    async fn unpin(
        &self,
        user: &UserId,
        kind: ConversationPinKind,
        target: Uuid,
    ) -> Result<ConversationPins, RepositoryError> {
        let mut tx = begin_pins_tx(&self.pool, user).await?;
        sqlx::query!(
            r#"DELETE FROM conversation_pins WHERE user_id = $1 AND kind = $2 AND target_id = $3"#,
            user.uuid(),
            kind.as_str(),
            target,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let pins = load_conversation_pins(&mut *tx, user).await?;
        save_pins_event(&mut tx, user, &pins).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(pins)
    }
}

async fn begin_pins_tx<'a>(pool: &'a Pool<Postgres>, user: &UserId) -> Result<Transaction<'a, Postgres>, RepositoryError> {
    let mut tx = pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 52))")
        .bind(format!("conversation_pins:{}", user.uuid()))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    Ok(tx)
}

async fn save_pins_event(
    tx: &mut Transaction<'_, Postgres>,
    user: &UserId,
    pins: &ConversationPins,
) -> Result<(), RepositoryError> {
    let record = OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: user.uuid(),
        aggregate_type: AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: "conversation_pins.updated".to_string(),
        payload: serde_json::json!({
            "user_id": user.uuid(),
            "dms": pins.dms,
            "communities": pins.communities,
        }),
        occurred_at: Utc::now(),
        request_id: None,
    };
    save_outbox_records(tx, std::slice::from_ref(&record)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use infrastructure_common::messaging::nats::producer::partitioned_subject;

    fn partition_of(subject: &str) -> String {
        subject.split('.').nth(2).unwrap().to_string()
    }

    #[test]
    fn channel_pin_events_share_the_channel_partition() {
        let channel = Uuid::new_v4();
        let pin = pin_event_record(PinScope::Channel(channel), Uuid::new_v4(), &UserId::new(Uuid::new_v4()), true, Utc::now(), &[]);
        let sent = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: Uuid::new_v4(),
            aggregate_type: AggregateType::Message,
            parent_id: Some(channel),
            parent_type: Some(AggregateType::Channel),
            event_type: "message.sent".to_string(),
            payload: serde_json::json!({}),
            occurred_at: Utc::now(),
            request_id: None,
        };
        let pin_subject = partitioned_subject("messaging", &pin, 16);
        assert!(pin_subject.ends_with(".message.pin_changed"));
        assert_eq!(partition_of(&pin_subject), partition_of(&partitioned_subject("messaging", &sent, 16)));
    }

    #[test]
    fn dm_pin_events_share_the_conversation_partition() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let scope = PinScope::dm(&UserId::new(a), &UserId::new(b));
        let pin = pin_event_record(scope, Uuid::new_v4(), &UserId::new(a), false, Utc::now(), &[]);
        let sent = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: Uuid::new_v4(),
            aggregate_type: AggregateType::DmMessage,
            parent_id: Some(b),
            parent_type: Some(AggregateType::User),
            event_type: "dm_message.sent".to_string(),
            payload: serde_json::json!({ "sender_id": b, "recipient_id": a }),
            occurred_at: Utc::now(),
            request_id: None,
        };
        let pin_subject = partitioned_subject("messaging", &pin, 16);
        let sent_subject = partitioned_subject("messaging", &sent, 16);
        assert_eq!(pin_subject.split('.').nth(1), sent_subject.split('.').nth(1));
        assert_eq!(partition_of(&pin_subject), partition_of(&sent_subject));
    }
}
