use async_trait::async_trait;
use messaging_domain::{friendship::Friendship, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, friendship::view::FriendshipView};
use domain::{shared::repository::error::RepositoryError, user::id::UserId};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::friendship_status::FriendshipStatusDto, error::map_sqlx_error_to_domain_error,
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
                    INNER JOIN messaging_user_profiles u ON u.user_id = f.friend_id
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
                INNER JOIN messaging_user_profiles u ON u.user_id = f.friend_id
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
