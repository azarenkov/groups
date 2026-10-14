use async_trait::async_trait;
use domain::user::id::UserId;
use messaging_domain::community::{
    id::CommunityId,
    invitation::{Invitation, InvitationId, InvitationRepository},
};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresInvitationRepository {
    pool: Pool<Postgres>,
}

impl PostgresInvitationRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvitationRepository for PostgresInvitationRepository {
    async fn find(&self, id: &InvitationId) -> Result<Option<Invitation>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT id, community_id, inviter_id, invitee_id, created_at
            FROM community_invitations
            WHERE id = $1
            "#,
            id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| {
            Invitation::restore(
                InvitationId::new(r.id),
                CommunityId::new(r.community_id),
                UserId::new(r.inviter_id),
                UserId::new(r.invitee_id),
                r.created_at,
            )
        }))
    }

    async fn is_pending(&self, community_id: &CommunityId, invitee_id: &UserId) -> Result<bool, RepositoryError> {
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM community_invitations WHERE community_id = $1 AND invitee_id = $2
            ) AS "exists!"
            "#,
            community_id.uuid(),
            invitee_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn pending_invitees(&self, community_id: &CommunityId) -> Result<Vec<UserId>, RepositoryError> {
        let ids = sqlx::query_scalar!(
            r#"SELECT invitee_id FROM community_invitations WHERE community_id = $1 ORDER BY created_at DESC"#,
            community_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(ids.into_iter().map(UserId::new).collect())
    }
}
