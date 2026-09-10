use async_trait::async_trait;
use domain::user::id::UserId;
use chrono::{DateTime, Utc};
use messaging_domain::{events::MessagingEvent, friendship::Friendship, friendship::block::BlockRelation, friendship::pair::FriendshipPair, friendship::status::FriendshipStatus, friendship::unit_of_work::FriendshipTransaction, friendship::unit_of_work::FriendshipUnitOfWork};

use crate::database::postgres::repositories::friendship::block_relation_for;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres, Transaction};

use crate::database::postgres::{
    dto::friendship_status::FriendshipStatusDto, error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};

pub struct PostgresFriendshipUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresFriendshipUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresFriendshipTransaction {
    tx: Transaction<'static, Postgres>,
}

#[async_trait]
impl FriendshipUnitOfWork for PostgresFriendshipUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn FriendshipTransaction>, RepositoryError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(Box::new(PostgresFriendshipTransaction { tx }))
    }
}

fn pair_key(a: &UserId, b: &UserId) -> String {
    let (lo, hi) = if a.uuid() <= b.uuid() { (a, b) } else { (b, a) };
    format!("{}:{}", lo.uuid(), hi.uuid())
}

#[async_trait]
impl FriendshipTransaction for PostgresFriendshipTransaction {
    async fn load_pair(&mut self, a: &UserId, b: &UserId) -> Result<Option<FriendshipPair>, RepositoryError> {
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtextextended($1::text, 43))",
            pair_key(a, b),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let rows = sqlx::query!(
            r#"
            SELECT
                user_id, friend_id,
                status AS "status: FriendshipStatusDto",
                created_at, updated_at
            FROM friendships
            WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)
            "#,
            a.uuid(),
            b.uuid(),
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let rows: Vec<Friendship> = rows
            .into_iter()
            .map(|r| Friendship {
                user_id: UserId::new(r.user_id),
                friend_id: UserId::new(r.friend_id),
                status: FriendshipStatus::from(r.status),
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect();
        let a_to_b = rows.iter().find(|r| r.user_id == *a);
        let b_to_a = rows.iter().find(|r| r.user_id == *b);
        Ok(FriendshipPair::from_rows(a_to_b, b_to_a))
    }

    async fn save_pair(&mut self, pair: &FriendshipPair) -> Result<(), RepositoryError> {
        let (a, b) = pair.users();
        let rows = pair.rows();
        let keep: Vec<uuid::Uuid> = rows.iter().map(|r| r.user_id.uuid()).collect();
        sqlx::query!(
            r#"
            DELETE FROM friendships
            WHERE ((user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1))
              AND NOT (user_id = ANY($3))
            "#,
            a.uuid(),
            b.uuid(),
            &keep,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        for row in rows {
            let status_dto = FriendshipStatusDto::from(row.status);
            sqlx::query!(
                r#"
                INSERT INTO friendships (user_id, friend_id, status, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (user_id, friend_id) DO UPDATE
                    SET status = EXCLUDED.status,
                        updated_at = EXCLUDED.updated_at
                "#,
                row.user_id.uuid(),
                row.friend_id.uuid(),
                status_dto as FriendshipStatusDto,
                row.created_at,
                row.updated_at,
            )
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }
        Ok(())
    }

    async fn block_relation(&mut self, viewer: &UserId, other: &UserId) -> Result<BlockRelation, RepositoryError> {
        block_relation_for(&mut *self.tx, viewer, other).await
    }

    async fn insert_block(&mut self, blocker: &UserId, blocked: &UserId, now: DateTime<Utc>) -> Result<bool, RepositoryError> {
        let inserted = sqlx::query(
            "INSERT INTO user_blocks (blocker_id, blocked_id, created_at) VALUES ($1, $2, $3) ON CONFLICT (blocker_id, blocked_id) DO NOTHING",
        )
        .bind(blocker.uuid())
        .bind(blocked.uuid())
        .bind(now)
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(inserted.rows_affected() > 0)
    }

    async fn delete_block(&mut self, blocker: &UserId, blocked: &UserId) -> Result<bool, RepositoryError> {
        let deleted = sqlx::query("DELETE FROM user_blocks WHERE blocker_id = $1 AND blocked_id = $2")
            .bind(blocker.uuid())
            .bind(blocked.uuid())
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(deleted.rows_affected() > 0)
    }

    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError> {
        let records = crate::outbox::mapping::to_records(events, request_id)?;
        save_outbox_records(&mut self.tx, &records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
