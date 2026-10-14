use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::{
    community::{id::CommunityId, invitation::InvitationId},
    inbox::{InboxCursor, InboxItem, InboxItemKind, InboxRepository, Notification},
};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub const NOTIFICATION_CREATED: &str = "notification.created";
pub const INBOX_UPDATED: &str = "inbox.updated";

pub(crate) async fn save_inbox_event(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    event_type: &str,
    notification_id: Option<Uuid>,
) -> Result<(), RepositoryError> {
    let mut payload = serde_json::json!({ "user_id": user_id });
    if let Some(id) = notification_id {
        payload["notification_id"] = serde_json::json!(id);
    }
    let record = OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: user_id,
        aggregate_type: AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: event_type.to_string(),
        payload,
        occurred_at: Utc::now(),
        request_id: None,
    };
    save_outbox_records(tx, std::slice::from_ref(&record)).await
}

pub struct PostgresInboxRepository {
    pool: Pool<Postgres>,
}

impl PostgresInboxRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

fn item_kind(
    kind: &str,
    community_id: Option<Uuid>,
    community_name: String,
    invitation_id: Option<Uuid>,
) -> Option<InboxItemKind> {
    match kind {
        "community_invitation" => Some(InboxItemKind::CommunityInvitation {
            invitation_id: InvitationId::new(invitation_id?),
            community_id: CommunityId::new(community_id?),
            community_name,
        }),
        "removed_from_community" => Some(InboxItemKind::RemovedFromCommunity {
            community_id: CommunityId::new(community_id?),
            community_name,
        }),
        "friend_request_received" => Some(InboxItemKind::FriendRequestReceived),
        "friend_request_accepted" => Some(InboxItemKind::FriendRequestAccepted),
        _ => None,
    }
}

#[async_trait]
impl InboxRepository for PostgresInboxRepository {
    async fn deliver(&self, notification: &Notification, source_event_id: &str) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let inserted = sqlx::query_scalar!(
            r#"
            INSERT INTO inbox_notifications
                (id, user_id, kind, actor_id, community_id, community_name, invitation_id, source_event_id, created_at)
            SELECT $1, $2, $3, $4, $5, (SELECT c.name FROM communities c WHERE c.id = $5), $6, $7, $8
            WHERE ($5::uuid IS NULL OR EXISTS (SELECT 1 FROM communities c WHERE c.id = $5))
              AND ($6::uuid IS NULL OR EXISTS (SELECT 1 FROM community_invitations i WHERE i.id = $6))
            ON CONFLICT (user_id, source_event_id) DO NOTHING
            RETURNING id
            "#,
            notification.id,
            notification.recipient_id.uuid(),
            notification.kind.as_str(),
            notification.kind.actor_id().uuid(),
            notification.kind.community_id().map(|c| c.uuid()),
            notification.kind.invitation_id().map(|i| i.uuid()),
            source_event_id,
            notification.created_at,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if let Some(id) = inserted {
            save_inbox_event(&mut tx, notification.recipient_id.uuid(), NOTIFICATION_CREATED, Some(id)).await?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(inserted.is_some())
    }

    async fn withdraw_invitation(&self, recipient_id: &UserId, invitation_id: &InvitationId) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let removed = sqlx::query!(
            "DELETE FROM inbox_notifications WHERE user_id = $1 AND invitation_id = $2",
            recipient_id.uuid(),
            invitation_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();
        save_inbox_event(&mut tx, recipient_id.uuid(), INBOX_UPDATED, None).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(removed > 0)
    }

    async fn list(&self, user_id: &UserId, before: Option<&InboxCursor>, limit: i64) -> Result<Vec<InboxItem>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT n.id, n.kind, n.actor_id, COALESCE(a.name, '') AS "actor_name!",
                   n.community_id, COALESCE(c.name, n.community_name, '') AS "community_name!",
                   n.invitation_id, n.created_at, n.read_at IS NOT NULL AS "read!"
            FROM inbox_notifications n
            LEFT JOIN messaging_user_profiles a ON a.user_id = n.actor_id
            LEFT JOIN communities c ON c.id = n.community_id
            WHERE n.user_id = $1
              AND NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = n.actor_id)
              AND NOT EXISTS (
                  SELECT 1 FROM user_blocks b
                  WHERE (b.blocker_id = $1 AND b.blocked_id = n.actor_id)
                     OR (b.blocker_id = n.actor_id AND b.blocked_id = $1)
              )
              AND ($2::timestamptz IS NULL OR (n.created_at, n.id) < ($2, $3))
            ORDER BY n.created_at DESC, n.id DESC
            LIMIT $4
            "#,
            user_id.uuid(),
            before.map(|c| c.created_at),
            before.map(|c| c.id),
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                Some(InboxItem {
                    id: r.id,
                    kind: item_kind(&r.kind, r.community_id, r.community_name, r.invitation_id)?,
                    actor_id: UserId::new(r.actor_id),
                    actor_name: r.actor_name,
                    created_at: r.created_at,
                    read: r.read,
                })
            })
            .collect())
    }

    async fn unread_count(&self, user_id: &UserId) -> Result<i64, RepositoryError> {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM inbox_notifications n
            WHERE n.user_id = $1
              AND n.read_at IS NULL
              AND NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = n.actor_id)
              AND NOT EXISTS (
                  SELECT 1 FROM user_blocks b
                  WHERE (b.blocker_id = $1 AND b.blocked_id = n.actor_id)
                     OR (b.blocker_id = n.actor_id AND b.blocked_id = $1)
              )
            "#,
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn mark_read(&self, user_id: &UserId, ids: Option<&[Uuid]>, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let updated = sqlx::query!(
            r#"
            UPDATE inbox_notifications SET read_at = $3
            WHERE user_id = $1 AND read_at IS NULL AND ($2::uuid[] IS NULL OR id = ANY($2))
            "#,
            user_id.uuid(),
            ids as Option<&[Uuid]>,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();
        if updated > 0 {
            save_inbox_event(&mut tx, user_id.uuid(), INBOX_UPDATED, None).await?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }

    async fn expire(&self, before: DateTime<Utc>, kept_kinds: &[&str], limit: i64) -> Result<u64, RepositoryError> {
        let kept: Vec<String> = kept_kinds.iter().map(|k| k.to_string()).collect();
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let owners = sqlx::query_scalar!(
            r#"
            WITH doomed AS (
                SELECT id FROM inbox_notifications
                WHERE created_at < $1 AND NOT (kind = ANY($2))
                ORDER BY created_at
                LIMIT $3
                FOR UPDATE SKIP LOCKED
            )
            DELETE FROM inbox_notifications n USING doomed
            WHERE n.id = doomed.id
            RETURNING n.user_id
            "#,
            before,
            &kept,
            limit,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        announce_pruned(tx, owners).await
    }

    async fn trim(&self, keep_per_user: i64, kept_kinds: &[&str], limit: i64) -> Result<u64, RepositoryError> {
        let kept: Vec<String> = kept_kinds.iter().map(|k| k.to_string()).collect();
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let owners = sqlx::query_scalar!(
            r#"
            WITH crowded AS (
                SELECT user_id FROM inbox_notifications
                WHERE NOT (kind = ANY($2))
                GROUP BY user_id
                HAVING COUNT(*) > $1
            ),
            ranked AS (
                SELECT n.id, row_number() OVER (PARTITION BY n.user_id ORDER BY n.created_at DESC, n.id DESC) AS position
                FROM inbox_notifications n
                JOIN crowded c ON c.user_id = n.user_id
                WHERE NOT (n.kind = ANY($2))
            ),
            doomed AS (
                SELECT n.id FROM inbox_notifications n
                JOIN ranked r ON r.id = n.id
                WHERE r.position > $1
                LIMIT $3
                FOR UPDATE OF n SKIP LOCKED
            )
            DELETE FROM inbox_notifications n USING doomed
            WHERE n.id = doomed.id
            RETURNING n.user_id
            "#,
            keep_per_user,
            &kept,
            limit,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        announce_pruned(tx, owners).await
    }
}

async fn announce_pruned(mut tx: Transaction<'_, Postgres>, owners: Vec<Uuid>) -> Result<u64, RepositoryError> {
    let removed = owners.len() as u64;
    let affected: std::collections::BTreeSet<Uuid> = owners.into_iter().collect();
    for user_id in affected {
        save_inbox_event(&mut tx, user_id, INBOX_UPDATED, None).await?;
    }
    tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
    Ok(removed)
}
