use async_trait::async_trait;
use messaging_domain::{community::id::CommunityId, community::role::CommunityRole, community::role::CommunityRoleRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresCommunityRoleRepository {
    pool: Pool<Postgres>,
}

impl PostgresCommunityRoleRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CommunityRoleRepository for PostgresCommunityRoleRepository {
    async fn list_roles(&self, community_id: &CommunityId) -> Result<Vec<CommunityRole>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, community_id, name, color, permissions, position, created_at
            FROM community_roles
            WHERE community_id = $1
            ORDER BY position DESC, created_at ASC
            "#,
            community_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| CommunityRole {
                id: r.id,
                community_id: r.community_id,
                name: r.name,
                color: r.color,
                permissions: messaging_domain::community::role::Permissions::from_bits_retain(r.permissions),
                position: r.position,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn member_role_ids(
        &self,
        community_id: &CommunityId,
    ) -> Result<Vec<(Uuid, Uuid)>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT mr.user_id, mr.role_id
            FROM community_member_roles mr
            INNER JOIN community_members m
                ON m.community_id = mr.community_id AND m.user_id = mr.user_id
            WHERE mr.community_id = $1
            "#,
            community_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|r| (r.user_id, r.role_id)).collect())
    }

    async fn user_permissions(
        &self,
        community_id: &CommunityId,
        user_id: &UserId,
    ) -> Result<messaging_domain::community::role::Permissions, RepositoryError> {
        let perms: Option<i64> = sqlx::query_scalar!(
            r#"
            SELECT bit_or(r.permissions) AS "perms"
            FROM community_member_roles mr
            INNER JOIN community_roles r ON r.id = mr.role_id
            INNER JOIN community_members m
                ON m.community_id = mr.community_id AND m.user_id = mr.user_id
            WHERE mr.community_id = $1 AND mr.user_id = $2
            "#,
            community_id.uuid(),
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(messaging_domain::community::role::Permissions::from_bits_retain(perms.unwrap_or(0)))
    }
}
