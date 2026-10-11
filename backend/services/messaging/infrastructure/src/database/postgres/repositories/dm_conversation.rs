use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::dm_conversation::{ClearScope, DmConversationRepository};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::{
        community_unit_of_work::lock_user, friendship_unit_of_work::pair_key, outbox_record::save_outbox_records,
    },
};
use crate::outbox::storage::release_storage_keys;

pub struct PostgresDmConversationRepository {
    pool: Pool<Postgres>,
}

impl PostgresDmConversationRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub(crate) fn cleared_event_record(
    actor: &UserId,
    peer: &UserId,
    scope: ClearScope,
    cleared_at: DateTime<Utc>,
) -> OutboxRecord {
    let recipients = match scope {
        ClearScope::Me => vec![actor.uuid()],
        ClearScope::Everyone => vec![actor.uuid(), peer.uuid()],
    };
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: actor.uuid(),
        aggregate_type: AggregateType::DmMessage,
        parent_id: None,
        parent_type: None,
        event_type: "dm_conversation.cleared".to_string(),
        payload: serde_json::json!({
            "sender_id": actor.uuid(),
            "recipient_id": peer.uuid(),
            "scope": scope.as_str(),
            "cleared_at": cleared_at,
            "recipients": recipients,
        }),
        occurred_at: cleared_at,
        request_id: None,
    }
}

pub(crate) fn hidden_event_record(user: &UserId, peer: &UserId, hidden: bool) -> OutboxRecord {
    let occurred_at = Utc::now();
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: user.uuid(),
        aggregate_type: AggregateType::User,
        parent_id: Some(peer.uuid()),
        parent_type: Some(AggregateType::User),
        event_type: "dm_conversation.hidden_changed".to_string(),
        payload: serde_json::json!({
            "user_id": user.uuid(),
            "peer_id": peer.uuid(),
            "hidden": hidden,
            "occurred_at": occurred_at,
        }),
        occurred_at,
        request_id: None,
    }
}

async fn lock_pair(tx: &mut Transaction<'_, Postgres>, a: &UserId, b: &UserId) -> Result<(), RepositoryError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 43))")
        .bind(pair_key(a, b))
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    Ok(())
}

#[async_trait]
impl DmConversationRepository for PostgresDmConversationRepository {
    async fn clear(
        &self,
        actor: &UserId,
        peer: &UserId,
        scope: ClearScope,
    ) -> Result<DateTime<Utc>, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        lock_pair(&mut tx, actor, peer).await?;
        match scope {
            ClearScope::Me => {
                sqlx::query!(
                    r#"
                    INSERT INTO hidden_messages (user_id, message_id)
                    SELECT $1, m.id FROM dm_messages m
                    WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                      AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                    ON CONFLICT (user_id, message_id) DO NOTHING
                    "#,
                    actor.uuid(),
                    peer.uuid(),
                )
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
            ClearScope::Everyone => {
                let removed_keys = sqlx::query_scalar!(
                    r#"
                    DELETE FROM dm_message_attachments a
                    USING dm_messages m
                    WHERE a.dm_message_id = m.id
                      AND LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                      AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                    RETURNING a.storage_key
                    "#,
                    actor.uuid(),
                    peer.uuid(),
                )
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                release_storage_keys(&mut tx, &removed_keys).await?;
                sqlx::query!(
                    r#"
                    DELETE FROM dm_messages
                    WHERE LEAST(sender_id, recipient_id) = LEAST($1::uuid, $2::uuid)
                      AND GREATEST(sender_id, recipient_id) = GREATEST($1::uuid, $2::uuid)
                    "#,
                    actor.uuid(),
                    peer.uuid(),
                )
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
        }
        let cleared_at = Utc::now();
        let record = cleared_event_record(actor, peer, scope, cleared_at);
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(cleared_at)
    }

    async fn set_hidden(&self, user: &UserId, peer: &UserId, hidden: bool) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        if hidden {
            lock_user(&mut tx, peer).await?;
            let inserted = sqlx::query!(
                r#"
                INSERT INTO hidden_dm_chats (user_id, peer_id)
                SELECT $1, $2
                WHERE EXISTS (SELECT 1 FROM messaging_user_profiles u WHERE u.user_id = $2)
                  AND NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = $2)
                ON CONFLICT (user_id, peer_id) DO UPDATE SET hidden_at = hidden_dm_chats.hidden_at
                "#,
                user.uuid(),
                peer.uuid(),
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if inserted.rows_affected() == 0 {
                return Err(RepositoryError::NotFound);
            }
        } else {
            sqlx::query!(
                r#"DELETE FROM hidden_dm_chats WHERE user_id = $1 AND peer_id = $2"#,
                user.uuid(),
                peer.uuid(),
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }
        let record = hidden_event_record(user, peer, hidden);
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn list_hidden(&self, user: &UserId) -> Result<Vec<Uuid>, RepositoryError> {
        sqlx::query_scalar!(
            r#"SELECT peer_id FROM hidden_dm_chats WHERE user_id = $1 ORDER BY hidden_at"#,
            user.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use infrastructure_common::messaging::nats::producer::partitioned_subject;

    #[test]
    fn cleared_event_shares_the_conversation_partition_with_messages() {
        let (a, b) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let cleared = cleared_event_record(&a, &b, ClearScope::Everyone, Utc::now());
        let sent = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: Uuid::new_v4(),
            aggregate_type: AggregateType::DmMessage,
            parent_id: Some(b.uuid()),
            parent_type: Some(AggregateType::User),
            event_type: "dm_message.sent".to_string(),
            payload: serde_json::json!({ "sender_id": b.uuid(), "recipient_id": a.uuid() }),
            occurred_at: Utc::now(),
            request_id: None,
        };
        let cleared_subject = partitioned_subject("messaging", &cleared, 16);
        let sent_subject = partitioned_subject("messaging", &sent, 16);
        assert!(cleared_subject.ends_with(".dm_conversation.cleared"));
        assert_eq!(
            cleared_subject.rsplitn(3, '.').nth(2),
            sent_subject.rsplitn(3, '.').nth(2),
        );
    }

    #[test]
    fn clearing_for_me_only_notifies_me() {
        let (a, b) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let mine = cleared_event_record(&a, &b, ClearScope::Me, Utc::now());
        assert_eq!(mine.payload["recipients"], serde_json::json!([a.uuid()]));
        let both = cleared_event_record(&a, &b, ClearScope::Everyone, Utc::now());
        assert_eq!(both.payload["recipients"], serde_json::json!([a.uuid(), b.uuid()]));
    }

    #[test]
    fn hidden_event_goes_to_the_user_stream() {
        let (a, b) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let subject = partitioned_subject("messaging", &hidden_event_record(&a, &b, true), 16);
        assert!(subject.starts_with("messaging.user."));
        assert!(subject.ends_with(".dm_conversation.hidden_changed"));
    }
}
