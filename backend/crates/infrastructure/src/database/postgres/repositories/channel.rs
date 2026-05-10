use std::collections::HashMap;

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use domain::{
    channel::{
        Channel, id::ChannelId, kind::ChannelKind, max_paticipants::MaxParticipants,
        name::ChannelName, participant_mute::ParticipantMute, repository::ChannelRepository,
    },
    server::id::ServerId,
    shared::{
        outbox::outbox_record::OutboxRecord, repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{
        channel::ChannelDto, kind::ChannelKindDto, muted_participant::MutedParticipantDto,
        visibility::ChannelVisibilityDto,
    },
    error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
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

        let kind_dto = ChannelKindDto::from(channel.kind());
        let visibility_dto = ChannelVisibilityDto::from(&channel.visibility());

        sqlx::query!(
            r#"
            INSERT INTO channels (
                id, server_id, owner_id, name, max_participants,
                kind, visibility, password_hash, created_at, updated_at, last_join
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            channel.id().uuid(),
            channel.server_id().uuid(),
            channel.owner_id().uuid(),
            channel.name().as_str(),
            channel.max_participants().value() as i16,
            kind_dto as ChannelKindDto,
            visibility_dto as ChannelVisibilityDto,
            channel.password_hash().map(|ph| ph.as_str()),
            channel.created_at(),
            channel.updated_at(),
            channel.last_join(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        for mute in channel.muted_participants_list() {
            sqlx::query!(
                r#"
                INSERT INTO channel_muted_participants
                    (channel_id, user_id, muted_by, muted_at)
                VALUES ($1, $2, $3, $4)
                "#,
                channel.id().uuid(),
                mute.user_id.uuid(),
                mute.muted_by.uuid(),
                mute.muted_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
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

        let result = sqlx::query!(
            r#"
            UPDATE channels
            SET last_join = GREATEST(last_join, $2)
            WHERE id = $1
            "#,
            channel.id().uuid(),
            channel.last_join(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn record_events(
        &self,
        channel_id: &ChannelId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        if events.is_empty() {
            return Ok(());
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let exists = sqlx::query!(
            r#"
            SELECT 1 AS "anchor!"
            FROM channels
            WHERE id = $1
            FOR SHARE
            "#,
            channel_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if exists.is_none() {
            return Err(RepositoryError::NotFound);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
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

        let result = sqlx::query!("DELETE FROM channels WHERE id = $1", channel_id.uuid())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn mute_participant_with_events(
        &self,
        channel_id: &ChannelId,
        target: &UserId,
        muted_by: &UserId,
        muted_at: DateTime<Utc>,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
            INSERT INTO channel_muted_participants
                (channel_id, user_id, muted_by, muted_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (channel_id, user_id) DO NOTHING
            "#,
            channel_id.uuid(),
            target.uuid(),
            muted_by.uuid(),
            muted_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::AlreadyExists);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn unmute_participant_with_events(
        &self,
        channel_id: &ChannelId,
        target: &UserId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
            DELETE FROM channel_muted_participants
            WHERE channel_id = $1 AND user_id = $2
            "#,
            channel_id.uuid(),
            target.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn find_channel_by_id(&self, id: &ChannelId) -> Result<Channel, RepositoryError> {
        let dto = sqlx::query_as!(
            ChannelDto,
            r#"
            SELECT id, server_id, owner_id, name, max_participants,
                   kind AS "kind: ChannelKindDto",
                   visibility AS "visibility: ChannelVisibilityDto",
                   password_hash, created_at, updated_at, last_join
            FROM channels
            WHERE id = $1
            "#,
            id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        let muted_dtos = sqlx::query_as!(
            MutedParticipantDto,
            r#"
            SELECT channel_id, user_id, muted_by, muted_at
            FROM channel_muted_participants
            WHERE channel_id = $1
            "#,
            id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let kind: ChannelKind = dto.kind.into();
        let muted_participants = if kind.supports_voice() {
            let map: HashMap<UserId, ParticipantMute> = muted_dtos
                .into_iter()
                .map(|dto| {
                    let mute: ParticipantMute = dto.into();
                    (mute.user_id.clone(), mute)
                })
                .collect();
            Some(map)
        } else {
            None
        };

        let channel = Channel::restore(
            ChannelId::new(dto.id),
            ServerId::new(dto.server_id),
            UserId::new(dto.owner_id),
            ChannelName::new(dto.name),
            MaxParticipants::new(dto.max_participants as u8),
            kind,
            dto.visibility.into(),
            dto.password_hash.map(PasswordHash::new),
            dto.created_at,
            dto.updated_at,
            muted_participants,
            dto.last_join,
        );

        Ok(channel)
    }
}
