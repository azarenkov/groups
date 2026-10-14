use async_trait::async_trait;
use domain::user::id::UserId;
use messaging_domain::community::invitation::LegacyInvitations;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::friendship_unit_of_work::pair_key,
    repositories::inbox::{INBOX_UPDATED, NOTIFICATION_CREATED, save_inbox_event},
};

pub struct PostgresLegacyInvitations {
    pool: Pool<Postgres>,
}

impl PostgresLegacyInvitations {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LegacyInvitations for PostgresLegacyInvitations {
    async fn adopt(&self, limit: i64) -> Result<u64, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let taken = sqlx::query!(
            r#"
            WITH keys AS (
                SELECT DISTINCT invite_community_id, recipient_id FROM dm_messages
                WHERE invite_community_id IS NOT NULL
                ORDER BY invite_community_id, recipient_id
                LIMIT $1
            ),
            legacy AS (
                SELECT d.id FROM dm_messages d
                JOIN keys k ON k.invite_community_id = d.invite_community_id AND k.recipient_id = d.recipient_id
                ORDER BY d.id
                FOR UPDATE OF d
            )
            DELETE FROM dm_messages m USING legacy
            WHERE m.id = legacy.id
            RETURNING m.invite_community_id AS "community_id!", m.sender_id, m.recipient_id, m.created_at
            "#,
            limit,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if taken.is_empty() {
            return Ok(0);
        }
        let pairs: std::collections::BTreeSet<String> =
            taken.iter().map(|t| pair_key(&UserId::new(t.sender_id), &UserId::new(t.recipient_id))).collect();
        for key in pairs {
            sqlx::query!("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 43))", key)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
        }
        let notified = sqlx::query!(
            r#"
            WITH legacy AS (
                SELECT DISTINCT ON (m.invite_community_id, m.recipient_id)
                       m.invite_community_id AS community_id, m.sender_id, m.recipient_id, m.created_at
                FROM (
                    SELECT o.invite_community_id, o.sender_id, o.recipient_id, o.created_at
                    FROM unnest($1::uuid[], $2::uuid[], $3::uuid[], $4::timestamptz[])
                        AS o(invite_community_id, sender_id, recipient_id, created_at)
                ) m
                WHERE NOT EXISTS (
                    SELECT 1 FROM community_members cm
                    WHERE cm.community_id = m.invite_community_id AND cm.user_id = m.recipient_id
                )
                AND NOT EXISTS (
                    SELECT 1 FROM user_blocks b
                    WHERE (b.blocker_id = m.sender_id AND b.blocked_id = m.recipient_id)
                       OR (b.blocker_id = m.recipient_id AND b.blocked_id = m.sender_id)
                )
                ORDER BY m.invite_community_id, m.recipient_id, m.created_at DESC
            ),
            issued AS (
                INSERT INTO community_invitations (id, community_id, inviter_id, invitee_id, created_at)
                SELECT gen_random_uuid(), l.community_id, l.sender_id, l.recipient_id, l.created_at FROM legacy l
                ON CONFLICT (community_id, invitee_id) DO NOTHING
                RETURNING id, community_id, inviter_id, invitee_id, created_at
            )
            INSERT INTO inbox_notifications (id, user_id, kind, actor_id, community_id, invitation_id, source_event_id, created_at)
            SELECT gen_random_uuid(), i.invitee_id, 'community_invitation', i.inviter_id, i.community_id, i.id, 'migrated:' || i.id, i.created_at
            FROM issued i
            ON CONFLICT (user_id, source_event_id) DO NOTHING
            RETURNING id, user_id
            "#,
            &taken.iter().map(|t| t.community_id).collect::<Vec<Uuid>>(),
            &taken.iter().map(|t| t.sender_id).collect::<Vec<Uuid>>(),
            &taken.iter().map(|t| t.recipient_id).collect::<Vec<Uuid>>(),
            &taken.iter().map(|t| t.created_at).collect::<Vec<_>>(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        for n in &notified {
            save_inbox_event(&mut tx, n.user_id, NOTIFICATION_CREATED, Some(n.id)).await?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(taken.len() as u64)
    }

    async fn release_blocked(&self, limit: i64) -> Result<u64, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let invitees = sqlx::query_scalar!(
            r#"
            WITH blocked AS (
                SELECT i.id FROM community_invitations i
                WHERE EXISTS (
                    SELECT 1 FROM user_blocks b
                    WHERE (b.blocker_id = i.inviter_id AND b.blocked_id = i.invitee_id)
                       OR (b.blocker_id = i.invitee_id AND b.blocked_id = i.inviter_id)
                )
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            DELETE FROM community_invitations i USING blocked
            WHERE i.id = blocked.id
            RETURNING i.invitee_id
            "#,
            limit,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let affected: std::collections::BTreeSet<_> = invitees.iter().copied().collect();
        for invitee in affected {
            save_inbox_event(&mut tx, invitee, INBOX_UPDATED, None).await?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(invitees.len() as u64)
    }
}
