use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use domain::{
    channel::{
        Channel, id::ChannelId, kind::ChannelKind, name::ChannelName,
        participant_mute::ParticipantMute, session_id::SessionId,
    },
    dm_message::id::DmMessageId,
    server::{
        Server, id::ServerId, invite_code::InviteCode, repository::ServerRepository,
    },
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
    repositories::{
        channel::{count_stream_viewers, stream_viewers_record},
        outbox_record::save_outbox_records,
    },
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
                INSERT INTO servers (id, name, owner_id, description, accent_color, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            server.id().uuid(),
            server.name().as_str(),
            server.owner_id().uuid(),
            server.description().map(|d| d.as_str()),
            server.accent_color(),
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
                FROM servers s
                INNER JOIN server_members sm ON sm.server_id = s.id
                WHERE sm.user_id = $1
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
                SELECT s.id, s.name, s.owner_id, s.description, s.accent_color,
                       s.created_at, s.updated_at
                FROM servers s
                INNER JOIN server_members sm ON sm.server_id = s.id
                WHERE sm.user_id = $1
                ORDER BY s.created_at DESC
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
                SELECT u.id, u.name, u.email, u.password_hash, u.created_at, u.updated_at, u.email_verified_at, u.phone_verified_at, u.locale
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

    async fn find_member_ids_by_server_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<UserId>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT user_id FROM server_members WHERE server_id = $1
            "#,
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows.into_iter().map(|r| UserId::new(r.user_id)).collect())
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
            SELECT channel_id, user_id, session_id, camera_on, screen_on
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
        let mut camera_by_channel: HashMap<Uuid, HashMap<UserId, HashSet<SessionId>>> =
            HashMap::new();
        let mut screen_by_channel: HashMap<Uuid, HashMap<UserId, HashSet<SessionId>>> =
            HashMap::new();
        for row in voice_rows {
            let user_id = UserId::new(row.user_id);
            let session_id = SessionId::new(row.session_id);
            if row.camera_on {
                camera_by_channel
                    .entry(row.channel_id)
                    .or_default()
                    .entry(user_id)
                    .or_default()
                    .insert(session_id.clone());
            }
            if row.screen_on {
                screen_by_channel
                    .entry(row.channel_id)
                    .or_default()
                    .entry(user_id)
                    .or_default()
                    .insert(session_id.clone());
            }
            voice_by_channel
                .entry(row.channel_id)
                .or_default()
                .entry(user_id)
                .or_default()
                .insert(session_id);
        }

        let text_rows = sqlx::query!(
            r#"
            SELECT channel_id, user_id
            FROM channel_text_participants
            WHERE channel_id = ANY($1)
            "#,
            &channel_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let mut text_by_channel: HashMap<Uuid, HashSet<UserId>> = HashMap::new();
        for row in text_rows {
            text_by_channel
                .entry(row.channel_id)
                .or_default()
                .insert(UserId::new(row.user_id));
        }

        let items: Vec<Channel> = channel_dtos
            .into_iter()
            .map(|dto| {
                let kind: ChannelKind = dto.kind.into();
                let (muted, voice, camera, screen) = if kind.supports_voice() {
                    (
                        Some(muted_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(voice_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(camera_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(screen_by_channel.remove(&dto.id).unwrap_or_default()),
                    )
                } else {
                    (None, None, None, None)
                };
                let text = if kind.supports_text() {
                    Some(text_by_channel.remove(&dto.id).unwrap_or_default())
                } else {
                    None
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
                    camera,
                    screen,
                    text,
                    dto.last_join,
                )
            })
            .collect();

        Ok(PaginatedResult::new(items, total, pagination))
    }

    async fn accept_invite(
        &self,
        invite_message_id: &DmMessageId,
        server_id: &ServerId,
        user_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let deleted = sqlx::query!(
            r#"
                DELETE FROM dm_messages
                WHERE id = $1 AND recipient_id = $2 AND invite_server_id = $3
            "#,
            invite_message_id.uuid(),
            user_id.uuid(),
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if deleted.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        sqlx::query!(
            r#"
                INSERT INTO server_members (server_id, user_id, joined_at)
                VALUES ($1, $2, NOW())
                ON CONFLICT (server_id, user_id) DO NOTHING
            "#,
            server_id.uuid(),
            user_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn remove_server_member(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
                DELETE FROM server_members
                WHERE server_id = $1 AND user_id = $2
            "#,
            server_id.uuid(),
            user_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        sqlx::query!(
            r#"
                DELETE FROM channel_text_participants
                WHERE user_id = $1
                  AND channel_id IN (SELECT id FROM channels WHERE server_id = $2)
            "#,
            user_id.uuid(),
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
                DELETE FROM channel_voice_participants
                WHERE user_id = $1
                  AND channel_id IN (SELECT id FROM channels WHERE server_id = $2)
            "#,
            user_id.uuid(),
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let watched = sqlx::query!(
            r#"
                SELECT DISTINCT v.channel_id, v.streamer_id
                FROM channel_stream_viewers v
                INNER JOIN channels c ON c.id = v.channel_id
                WHERE v.viewer_id = $1 AND c.server_id = $2 AND v.streamer_id <> $1
            "#,
            user_id.uuid(),
            server_id.uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let streamed = sqlx::query!(
            r#"
                SELECT DISTINCT v.channel_id
                FROM channel_stream_viewers v
                INNER JOIN channels c ON c.id = v.channel_id
                WHERE v.streamer_id = $1 AND c.server_id = $2
            "#,
            user_id.uuid(),
            server_id.uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
                DELETE FROM channel_stream_viewers
                WHERE (viewer_id = $1 OR streamer_id = $1)
                  AND channel_id IN (SELECT id FROM channels WHERE server_id = $2)
            "#,
            user_id.uuid(),
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let now = chrono::Utc::now();
        let mut viewer_events = Vec::new();
        for w in watched {
            let count = count_stream_viewers(&mut tx, w.channel_id, w.streamer_id).await?;
            viewer_events.push(stream_viewers_record(
                w.channel_id,
                server_id.uuid(),
                w.streamer_id,
                count,
                now,
                None,
            ));
        }
        for s in streamed {
            viewer_events.push(stream_viewers_record(
                s.channel_id,
                server_id.uuid(),
                user_id.uuid(),
                0,
                now,
                None,
            ));
        }
        save_outbox_records(&mut tx, &viewer_events).await?;

        sqlx::query!(
            r#"
                DELETE FROM channel_muted_participants
                WHERE user_id = $1
                  AND channel_id IN (SELECT id FROM channels WHERE server_id = $2)
            "#,
            user_id.uuid(),
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
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

    async fn find_server_by_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<Server>, RepositoryError> {
        let dto = sqlx::query_as!(
            ServerDto,
            r#"
                SELECT id, name, owner_id, description, accent_color,
                       created_at, updated_at
                FROM servers
                WHERE id = $1
            "#,
            server_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(dto.map(|dto| dto.into()))
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

    async fn create_invite_code(
        &self,
        server_id: &ServerId,
        created_by: &UserId,
        code: &str,
        max_uses: Option<i32>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<InviteCode, RepositoryError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO server_invite_codes (code, server_id, created_by, max_uses, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING code, server_id, created_by, max_uses, uses, expires_at, created_at
            "#,
            code,
            server_id.uuid(),
            created_by.uuid(),
            max_uses,
            expires_at,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(InviteCode {
            code: row.code,
            server_id: row.server_id,
            created_by: row.created_by,
            creator_name: None,
            max_uses: row.max_uses,
            uses: row.uses,
            expires_at: row.expires_at,
            created_at: row.created_at,
        })
    }

    async fn list_invite_codes(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<InviteCode>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                i.code, i.server_id, i.created_by, i.max_uses, i.uses,
                i.expires_at, i.created_at,
                u.name AS "creator_name?"
            FROM server_invite_codes i
            LEFT JOIN users u ON u.id = i.created_by
            WHERE i.server_id = $1
              AND i.revoked_at IS NULL
              AND (i.expires_at IS NULL OR i.expires_at > now())
              AND (i.max_uses IS NULL OR i.uses < i.max_uses)
            ORDER BY i.created_at DESC
            "#,
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| InviteCode {
                code: r.code,
                server_id: r.server_id,
                created_by: r.created_by,
                creator_name: r.creator_name,
                max_uses: r.max_uses,
                uses: r.uses,
                expires_at: r.expires_at,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn revoke_invite_code(
        &self,
        server_id: &ServerId,
        code: &str,
        requester: &UserId,
        is_owner: bool,
    ) -> Result<bool, RepositoryError> {
        let updated = sqlx::query!(
            r#"
            UPDATE server_invite_codes
            SET revoked_at = now()
            WHERE code = $1 AND server_id = $2 AND revoked_at IS NULL
              AND (created_by = $3 OR $4)
            "#,
            code,
            server_id.uuid(),
            requester.uuid(),
            is_owner,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(updated.rows_affected() > 0)
    }

    async fn redeem_invite_code(
        &self,
        code: &str,
        user_id: &UserId,
    ) -> Result<Option<ServerId>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let invite = sqlx::query!(
            r#"
            SELECT server_id
            FROM server_invite_codes
            WHERE code = $1
              AND revoked_at IS NULL
              AND (expires_at IS NULL OR expires_at > now())
              AND (max_uses IS NULL OR uses < max_uses)
            FOR UPDATE
            "#,
            code,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let Some(invite) = invite else {
            return Ok(None);
        };

        let inserted = sqlx::query!(
            r#"
            INSERT INTO server_members (server_id, user_id, joined_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (server_id, user_id) DO NOTHING
            "#,
            invite.server_id,
            user_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if inserted.rows_affected() > 0 {
            sqlx::query!(
                r#"UPDATE server_invite_codes SET uses = uses + 1 WHERE code = $1"#,
                code,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(Some(ServerId::new(invite.server_id)))
    }

    async fn update_server_details(
        &self,
        server_id: &ServerId,
        name: &str,
        description: Option<&str>,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let updated = sqlx::query!(
            r#"
            UPDATE servers
            SET name = $2, description = $3, updated_at = now()
            WHERE id = $1
            "#,
            server_id.uuid(),
            name,
            description,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if updated.rows_affected() == 0 {
            return Ok(false);
        }

        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }
}
