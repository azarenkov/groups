use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    friendship::{
        Friendship, repository::FriendshipRepository, status::FriendshipStatus, view::FriendshipView,
    },
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::friendship_status::FriendshipStatusDto, error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};

pub struct PostgresFriendshipRepository {
    pool: Pool<Postgres>,
}

impl PostgresFriendshipRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FriendshipRepository for PostgresFriendshipRepository {
    async fn find_pair(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<Option<Friendship>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT
                user_id, friend_id,
                status AS "status: FriendshipStatusDto",
                created_at, updated_at
            FROM friendships
            WHERE user_id = $1 AND friend_id = $2
            "#,
            user_id.uuid(),
            friend_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|r| Friendship {
            user_id: UserId::new(r.user_id),
            friend_id: UserId::new(r.friend_id),
            status: FriendshipStatus::from(r.status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    async fn save_request_with_events(
        &self,
        outgoing: &Friendship,
        incoming: &Friendship,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        for row in [outgoing, incoming] {
            let status_dto = FriendshipStatusDto::from(row.status);
            sqlx::query!(
                r#"
                INSERT INTO friendships (user_id, friend_id, status, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                row.user_id.uuid(),
                row.friend_id.uuid(),
                status_dto as FriendshipStatusDto,
                row.created_at,
                row.updated_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn accept_pair_with_events(
        &self,
        accepter: &UserId,
        sender: &UserId,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let now: DateTime<Utc> = Utc::now();

        let updated_accepter = sqlx::query!(
            r#"
            UPDATE friendships
            SET status = $3, updated_at = $4
            WHERE user_id = $1 AND friend_id = $2 AND status = $5
            "#,
            accepter.uuid(),
            sender.uuid(),
            FriendshipStatusDto::Accepted as FriendshipStatusDto,
            now,
            FriendshipStatusDto::PendingIncoming as FriendshipStatusDto,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();

        let updated_sender = sqlx::query!(
            r#"
            UPDATE friendships
            SET status = $3, updated_at = $4
            WHERE user_id = $1 AND friend_id = $2 AND status = $5
            "#,
            sender.uuid(),
            accepter.uuid(),
            FriendshipStatusDto::Accepted as FriendshipStatusDto,
            now,
            FriendshipStatusDto::PendingOutgoing as FriendshipStatusDto,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();

        if updated_accepter == 0 || updated_sender == 0 {
            tx.rollback()
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            return Ok(false);
        }

        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn delete_pair_with_events(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let deleted = sqlx::query!(
            r#"
            DELETE FROM friendships
            WHERE (user_id = $1 AND friend_id = $2)
               OR (user_id = $2 AND friend_id = $1)
            "#,
            user_id.uuid(),
            friend_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();

        if deleted == 0 {
            tx.rollback()
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            return Ok(false);
        }

        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn list_for_user(
        &self,
        user_id: &UserId,
        status: Option<FriendshipStatus>,
    ) -> Result<Vec<FriendshipView>, RepositoryError> {
        let rows = match status {
            Some(s) => {
                let status_dto = FriendshipStatusDto::from(s);
                sqlx::query!(
                    r#"
                    SELECT
                        f.user_id, f.friend_id,
                        f.status AS "status: FriendshipStatusDto",
                        u.name AS friend_name,
                        f.created_at, f.updated_at
                    FROM friendships f
                    INNER JOIN users u ON u.id = f.friend_id
                    WHERE f.user_id = $1 AND f.status = $2
                    ORDER BY f.updated_at DESC
                    "#,
                    user_id.uuid(),
                    status_dto as FriendshipStatusDto,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?
                .into_iter()
                .map(|r| FriendshipView {
                    user_id: UserId::new(r.user_id),
                    friend_id: UserId::new(r.friend_id),
                    friend_name: r.friend_name,
                    status: FriendshipStatus::from(r.status),
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
                .collect()
            }
            None => sqlx::query!(
                r#"
                SELECT
                    f.user_id, f.friend_id,
                    f.status AS "status: FriendshipStatusDto",
                    u.name AS friend_name,
                    f.created_at, f.updated_at
                FROM friendships f
                INNER JOIN users u ON u.id = f.friend_id
                WHERE f.user_id = $1
                ORDER BY f.updated_at DESC
                "#,
                user_id.uuid(),
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .into_iter()
            .map(|r| FriendshipView {
                user_id: UserId::new(r.user_id),
                friend_id: UserId::new(r.friend_id),
                friend_name: r.friend_name,
                status: FriendshipStatus::from(r.status),
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect(),
        };
        Ok(rows)
    }
}
