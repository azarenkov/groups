use async_trait::async_trait;

use domain::{
    channel::{Channel, id::ChannelId, name::ChannelName, repository::ChannelRepository},
    server::id::ServerId,
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{aggregate_type::AggregateTypeDto, channel::ChannelDto, channel_type::ChannelTypeDto},
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
    async fn save_channel_with_events(
        &self,
        channel: &Channel,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let channel_type_dto: ChannelTypeDto = channel.channel_type().into();

        sqlx::query!(
            r#"
               INSERT INTO channels (
                   id, server_id, owner_id, name, max_participants,
                   channel_type, password_hash, created_at, updated_at, last_join
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               "#,
            channel.id().uuid(),
            channel.server_id().uuid(),
            channel.owner_id().uuid(),
            channel.name().as_str(),
            channel.max_participants().value() as i16,
            channel_type_dto as ChannelTypeDto,
            channel.password_hash().map(|ph| ph.as_str()),
            channel.created_at(),
            channel.updated_at(),
            channel.last_join(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        for record in events {
            let aggregate_type = AggregateTypeDto::from(&record.aggregate_type);
            let parent_type = record.parent_type.as_ref().map(AggregateTypeDto::from);

            sqlx::query!(
                r#"
                   INSERT INTO outbox (
                       id, aggregate_id, aggregate_type,
                       parent_id, parent_type,
                       event_type, payload, occurred_at
                   )
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                   "#,
                record.id,
                record.aggregate_id,
                aggregate_type as AggregateTypeDto,
                record.parent_id,
                parent_type as Option<AggregateTypeDto>,
                record.event_type,
                record.payload,
                record.occurred_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn find_channel_name_by_id(
        &self,
        channel_id: &ChannelId,
    ) -> Result<ChannelName, RepositoryError> {
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
    async fn update_channel_with_events(
        &self,
        channel: &Channel,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let channel_type_dto: ChannelTypeDto = channel.channel_type().into();

        let result = sqlx::query!(
            r#"
               UPDATE channels
               SET name = $2,
                   max_participants = $3,
                   channel_type = $4,
                   password_hash = $5,
                   updated_at = $6,
                   last_join = $7
               WHERE id = $1
               "#,
            channel.id().uuid(),
            channel.name().as_str(),
            channel.max_participants().value() as i16,
            channel_type_dto as ChannelTypeDto,
            channel.password_hash().map(|ph| ph.as_str()),
            channel.updated_at(),
            channel.last_join(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        for record in events {
            let aggregate_type = AggregateTypeDto::from(&record.aggregate_type);
            let parent_type = record.parent_type.as_ref().map(AggregateTypeDto::from);

            sqlx::query!(
                r#"
                   INSERT INTO outbox (
                       id, aggregate_id, aggregate_type,
                       parent_id, parent_type,
                       event_type, payload, occurred_at
                   )
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                   "#,
                record.id,
                record.aggregate_id,
                aggregate_type as AggregateTypeDto,
                record.parent_id,
                parent_type as Option<AggregateTypeDto>,
                record.event_type,
                record.payload,
                record.occurred_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn find_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError> {
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

    async fn delete_channel_with_events(
        &self,
        channel_id: &ChannelId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!("DELETE FROM channels WHERE id = $1", channel_id.uuid())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        for record in events {
            let aggregate_type_dto = AggregateTypeDto::from(&record.aggregate_type);
            let parent_type_dto = record.parent_type.as_ref().map(AggregateTypeDto::from);

            sqlx::query!(
                r#"
                INSERT INTO outbox (
                    id, aggregate_id, aggregate_type,
                    parent_id, parent_type,
                    event_type, payload, occurred_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                record.id,
                record.aggregate_id,
                aggregate_type_dto as AggregateTypeDto,
                record.parent_id,
                parent_type_dto as Option<AggregateTypeDto>,
                record.event_type,
                record.payload,
                record.occurred_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }
}
