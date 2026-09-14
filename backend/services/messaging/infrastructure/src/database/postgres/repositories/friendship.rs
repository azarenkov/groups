use async_trait::async_trait;
use messaging_domain::{friendship::block::BlockRelations, friendship::Friendship, friendship::block::BlockRelation, friendship::block::BlockedUserView, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, friendship::view::FriendshipView};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
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

    async fn block_relation(
        &self,
        viewer: &UserId,
        other: &UserId,
    ) -> Result<BlockRelation, RepositoryError> {
        block_relation_for(&self.pool, viewer, other).await
    }

    async fn blockers_among(
        &self,
        viewer: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, RepositoryError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<uuid::Uuid> = candidates.iter().map(|c| c.uuid()).collect();
        let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT blocker_id FROM user_blocks WHERE blocked_id = $1 AND blocker_id = ANY($2)",
        )
        .bind(viewer.uuid())
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|(id,)| UserId::new(id)).collect())
    }

    async fn blocked_among(
        &self,
        blocker: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, RepositoryError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<uuid::Uuid> = candidates.iter().map(|c| c.uuid()).collect();
        let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT blocked_id FROM user_blocks WHERE blocker_id = $1 AND blocked_id = ANY($2)",
        )
        .bind(blocker.uuid())
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|(id,)| UserId::new(id)).collect())
    }

    async fn block_relations_among(
        &self,
        user: &UserId,
        candidates: &[UserId],
    ) -> Result<BlockRelations, RepositoryError> {
        if candidates.is_empty() {
            return Ok(BlockRelations::default());
        }
        let ids: Vec<uuid::Uuid> = candidates.iter().map(|c| c.uuid()).collect();
        let rows: Vec<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
            "SELECT blocker_id, blocked_id FROM user_blocks WHERE (blocker_id = $1 AND blocked_id = ANY($2)) OR (blocked_id = $1 AND blocker_id = ANY($2))",
        )
        .bind(user.uuid())
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let mut relations = BlockRelations::default();
        for (blocker, blocked) in rows {
            if blocker == user.uuid() {
                relations.blocking.push(UserId::new(blocked));
            } else {
                relations.blocked_by.push(UserId::new(blocker));
            }
        }
        Ok(relations)
    }

    async fn list_blocked(&self, blocker: &UserId) -> Result<Vec<BlockedUserView>, RepositoryError> {
        let rows: Vec<(uuid::Uuid, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
            "SELECT b.blocked_id, COALESCE(u.name, '') AS name, b.created_at FROM user_blocks b LEFT JOIN messaging_user_profiles u ON u.user_id = b.blocked_id WHERE b.blocker_id = $1 ORDER BY b.created_at DESC",
        )
        .bind(blocker.uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|(user_id, name, created_at)| BlockedUserView {
                user_id: UserId::new(user_id),
                name,
                created_at,
            })
            .collect())
    }

}

pub(crate) async fn block_relation_for<'e, E>(
    executor: E,
    viewer: &UserId,
    other: &UserId,
) -> Result<BlockRelation, RepositoryError>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT blocker_id FROM user_blocks WHERE (blocker_id = $1 AND blocked_id = $2) OR (blocker_id = $2 AND blocked_id = $1)",
    )
    .bind(viewer.uuid())
    .bind(other.uuid())
    .fetch_all(executor)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    let mut relation = BlockRelation::default();
    for (blocker,) in rows {
        if blocker == viewer.uuid() {
            relation.blocking = true;
        } else {
            relation.blocked_by = true;
        }
    }
    Ok(relation)
}
