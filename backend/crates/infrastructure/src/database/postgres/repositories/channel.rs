use async_trait::async_trait;

use domain::{
    channel::{
        Channel,
        repository::ChannelRepository,
        id::ChannelId, name::ChannelName,
    },
    server::id::ServerId,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{channel::ChannelDto, channel_type::ChannelTypeDto},
    error::map_sqlx_error_to_domain_error,
};

pub struct PostgresChannelRepository {
    pool: Pool<Postgres>,
}

impl PostgresChannelRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChannelRepository for PostgresChannelRepository {
    async fn save_channel(&self, channel: &Channel) -> Result<(), RepositoryError> {
        let room_type_dto: ChannelTypeDto = channel.channel_type().into();

        sqlx::query!(
            r#"
                INSERT INTO channels (id, server_id, owner_id, name, max_participants, channel_type, password_hash, created_at, updated_at, last_join)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            channel.id().uuid(),
            channel.server_id().uuid(),
            channel.owner_id().uuid(),
            channel.name().as_str(),
            channel.max_participants().value() as i16,
            room_type_dto as ChannelTypeDto,
            channel.password_hash().map(|ph| ph.as_str()),
            channel.created_at(),
            channel.updated_at(),
            channel.last_join()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn get_channel_name_by_id(&self, channel_id: &ChannelId) -> Result<ChannelName, RepositoryError> {
        let record = sqlx::query!(
            r#"
                 SELECT name
                 FROM channels
                 WHERE id = $1
             "#,
            channel_id.uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        match record {
            Some(row) => Ok(ChannelName::new(row.name)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn get_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError> {
        let dto = sqlx::query_as!(
            ChannelDto,
            r#"
                SELECT
                    id,
                    server_id,
                    owner_id,
                    name,
                    max_participants,
                    channel_type as "channel_type: ChannelTypeDto",
                    password_hash,
                    created_at,
                    updated_at,
                    last_join
                FROM channels
                WHERE id = $1
            "#,
            channel_id.uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        Ok(dto.into())
    }

    async fn channel_exists(&self, channel_id: &ChannelId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1) as "exists!"
            "#,
            channel_id.uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.exists)
    }

    async fn channel_exists_with_server_id(
        &self,
        channel_id: &ChannelId,
        server_id: &ServerId,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND server_id = $2) as "exists!"
            "#,
            channel_id.uuid(),
            server_id.uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.exists)
    }

    async fn delete_channel_by_owner(
        &self,
        channel_id: &ChannelId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                DELETE FROM channels WHERE id = $1 AND owner_id = $2
            "#,
            channel_id.uuid(),
            owner_id.uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.rows_affected() > 0)
    }
}
