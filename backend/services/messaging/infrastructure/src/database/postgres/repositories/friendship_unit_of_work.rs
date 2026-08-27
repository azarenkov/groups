use async_trait::async_trait;
use chrono::Utc;
use domain::user::id::UserId;
use ports::{outbox::record::OutboxRecord, repository::RepositoryError};
use messaging_domain::{friendship::Friendship, friendship::unit_of_work::FriendshipTransaction, friendship::unit_of_work::FriendshipUnitOfWork};
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

#[async_trait]
impl FriendshipTransaction for PostgresFriendshipTransaction {
    async fn insert_request_pair(
        &mut self,
        outgoing: &Friendship,
        incoming: &Friendship,
    ) -> Result<(), RepositoryError> {
        let mut pair = [outgoing, incoming];
        pair.sort_by(|a, b| {
            (a.user_id.uuid(), a.friend_id.uuid())
                .cmp(&(b.user_id.uuid(), b.friend_id.uuid()))
        });
        for row in pair {
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

    async fn accept_pair(&mut self, accepter: &UserId, sender: &UserId) -> Result<bool, RepositoryError> {
        let now = Utc::now();
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
        .execute(&mut *self.tx)
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
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();
        Ok(updated_accepter > 0 && updated_sender > 0)
    }

    async fn decline_request(&mut self, decliner: &UserId, sender: &UserId) -> Result<bool, RepositoryError> {
        let now = Utc::now();
        let updated_sender = sqlx::query!(
            r#"
            UPDATE friendships
            SET status = $3, updated_at = $4
            WHERE user_id = $1 AND friend_id = $2 AND status = $5
            "#,
            sender.uuid(),
            decliner.uuid(),
            FriendshipStatusDto::Declined as FriendshipStatusDto,
            now,
            FriendshipStatusDto::PendingOutgoing as FriendshipStatusDto,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();
        let deleted_self = sqlx::query!(
            r#"
            DELETE FROM friendships
            WHERE user_id = $1 AND friend_id = $2 AND status = $3
            "#,
            decliner.uuid(),
            sender.uuid(),
            FriendshipStatusDto::PendingIncoming as FriendshipStatusDto,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();
        Ok(updated_sender > 0 && deleted_self > 0)
    }

    async fn delete_pair(&mut self, user_id: &UserId, friend_id: &UserId) -> Result<bool, RepositoryError> {
        let deleted = sqlx::query!(
            r#"
            DELETE FROM friendships
            WHERE (user_id = $1 AND friend_id = $2)
               OR (user_id = $2 AND friend_id = $1)
            "#,
            user_id.uuid(),
            friend_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .rows_affected();
        Ok(deleted > 0)
    }

    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError> {
        save_outbox_records(&mut self.tx, records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
