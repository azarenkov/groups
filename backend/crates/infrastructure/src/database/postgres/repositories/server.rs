use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use domain::{
    channel::{
        Channel, id::ChannelId, kind::ChannelKind, name::ChannelName,
        participant_mute::ParticipantMute, session_id::SessionId,
    },
    server::{Server, id::ServerId, repository::ServerRepository},
    shared::{
        outbox::outbox_record::OutboxRecord,
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::{User, id::UserId},
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    dto::{
        channel::ChannelDto, kind::ChannelKindDto, muted_participant::MutedParticipantDto,
        server::ServerDto, user::UserDto, visibility::ChannelVisibilityDto,
    },
    error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};

pub struct PostgresServerRepository {
    pool: Pool<Postgres>,
}

impl PostgresServerRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServerRepository for PostgresServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                SELECT EXISTS(SELECT 1 FROM servers WHERE id = $1) as "exists!"
            "#,
            server_id.uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.exists)
    }

    async fn save_server_with_events(
        &self,
        server: &Server,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
                INSERT INTO servers (id, name, owner_id, description, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            server.id().uuid(),
            server.name().as_str(),
            server.owner_id().uuid(),
            server.description().map(|d| d.as_str()),
            server.created_at(),
            server.updated_at()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
                INSERT INTO server_members (server_id, user_id, joined_at)
                VALUES ($1, $2, NOW())
            "#,
            server.id().uuid(),
            server.owner_id().uuid()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn find_servers_by_user_id(
        &self,
        user_id: &UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Server>, RepositoryError> {
        let total = sqlx::query!(
            r#"
                SELECT COUNT(*) as "count!"
                FROM servers
                WHERE owner_id = $1
            "#,
            user_id.uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .count;

        let servers = sqlx::query_as!(
            ServerDto,
            r#"
                SELECT id, name, owner_id, description, created_at, updated_at
                FROM servers
                WHERE owner_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
            "#,
            user_id.uuid(),
            pagination.limit(),
            pagination.offset()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let items = servers.into_iter().map(|dto| dto.into()).collect();
        Ok(PaginatedResult::new(items, total, pagination))
    }

    async fn find_users_by_server_id(
        &self,
        server_id: &ServerId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<User>, RepositoryError> {
        let total = sqlx::query!(
            r#"
                SELECT COUNT(*) as "count!"
                FROM server_members
                WHERE server_id = $1
            "#,
            server_id.uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .count;

        let users = sqlx::query_as!(
            UserDto,
            r#"
                SELECT u.id, u.name, u.email, u.password_hash, u.created_at, u.updated_at, u.email_verified_at, u.phone_verified_at
                FROM users u
                INNER JOIN server_members sm ON sm.user_id = u.id
                WHERE sm.server_id = $1
                ORDER BY sm.joined_at ASC
                LIMIT $2 OFFSET $3
            "#,
            server_id.uuid(),
            pagination.limit(),
            pagination.offset()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let items = users.into_iter().map(|dto| dto.into()).collect();
        Ok(PaginatedResult::new(items, total, pagination))
    }

    async fn find_channels_by_server_id(
        &self,
        server_id: &ServerId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Channel>, RepositoryError> {
        let total = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM channels
            WHERE server_id = $1
            "#,
            server_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .count;

        let channel_dtos = sqlx::query_as!(
            ChannelDto,
            r#"
            SELECT
                id, server_id, owner_id, name,
                kind as "kind: ChannelKindDto",
                visibility as "visibility: ChannelVisibilityDto",
                password_hash, created_at, updated_at, last_join
            FROM channels
            WHERE server_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            server_id.uuid(),
            pagination.limit(),
            pagination.offset(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let channel_ids: Vec<Uuid> = channel_dtos.iter().map(|c| c.id).collect();

        let muted_dtos = sqlx::query_as!(
            MutedParticipantDto,
            r#"
            SELECT channel_id, user_id, muted_by, muted_at
            FROM channel_muted_participants
            WHERE channel_id = ANY($1)
            "#,
            &channel_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let mut muted_by_channel: HashMap<Uuid, HashMap<UserId, ParticipantMute>> = HashMap::new();
        for dto in muted_dtos {
            let user_id = UserId::new(dto.user_id);
            let mute = ParticipantMute::new(user_id, UserId::new(dto.muted_by), dto.muted_at);
            muted_by_channel
                .entry(dto.channel_id)
                .or_default()
                .insert(user_id, mute);
        }

        let voice_rows = sqlx::query!(
            r#"
            SELECT channel_id, user_id, session_id
            FROM channel_voice_participants
            WHERE channel_id = ANY($1)
            "#,
            &channel_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let mut voice_by_channel: HashMap<Uuid, HashMap<UserId, HashSet<SessionId>>> =
            HashMap::new();
        for row in voice_rows {
            voice_by_channel
                .entry(row.channel_id)
                .or_default()
                .entry(UserId::new(row.user_id))
                .or_default()
                .insert(SessionId::new(row.session_id));
        }

        let items: Vec<Channel> = channel_dtos
            .into_iter()
            .map(|dto| {
                let kind: ChannelKind = dto.kind.into();
                let (muted, voice) = if kind.supports_voice() {
                    (
                        Some(muted_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(voice_by_channel.remove(&dto.id).unwrap_or_default()),
                    )
                } else {
                    (None, None)
                };
                Channel::restore(
                    ChannelId::new(dto.id),
                    ServerId::new(dto.server_id),
                    UserId::new(dto.owner_id),
                    ChannelName::new(dto.name),
                    kind,
                    dto.visibility.into(),
                    dto.password_hash.map(PasswordHash::new),
                    dto.created_at,
                    dto.updated_at,
                    muted,
                    voice,
                    dto.last_join,
                )
            })
            .collect();

        Ok(PaginatedResult::new(items, total, pagination))
    }

    async fn save_new_member_server(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                INSERT INTO server_members (server_id, user_id, joined_at)
                VALUES ($1, $2, NOW())
                ON CONFLICT (server_id, user_id) DO NOTHING
            "#,
            server_id.uuid(),
            user_id.uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn delete_server_by_owner_with_events(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
                DELETE FROM servers WHERE id = $1 AND owner_id = $2
            "#,
            server_id.uuid(),
            owner_id.uuid()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(true)
    }

    async fn find_server_owner(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<UserId>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT owner_id FROM servers WHERE id = $1"#,
            server_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|r| UserId::new(r.owner_id)))
    }

    async fn is_server_member(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM server_members
                WHERE server_id = $1 AND user_id = $2
            ) AS "exists!"
            "#,
            server_id.uuid(),
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.exists)
    }
}
