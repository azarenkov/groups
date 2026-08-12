use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use domain::{
    channel::{
        Channel, MediaChange, VoiceChange,
        events::participant::{ConnectionAbortReason, LeaveReason, MediaSource},
        id::ChannelId,
        kind::ChannelKind,
        name::ChannelName,
        participant_mute::ParticipantMute,
        repository::{ChannelOperationError, ChannelRepository},
        session_id::SessionId,
    },
    server::id::ServerId,
    shared::{
        outbox::{outbox_record::OutboxRecord, to_outbox::ToOutboxRecord},
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::{User, id::UserId},
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{
        channel::ChannelDto, kind::ChannelKindDto, muted_participant::MutedParticipantDto,
        user::UserDto, visibility::ChannelVisibilityDto,
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
                id, server_id, owner_id, name,
                kind, visibility, password_hash, created_at, updated_at, last_join
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            channel.id().uuid(),
            channel.server_id().uuid(),
            channel.owner_id().uuid(),
            channel.name().as_str(),
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

        if let Some(participants) = channel.text_participants() {
            for user_id in participants {
                sqlx::query!(
                    r#"
                    INSERT INTO channel_text_participants (channel_id, user_id, joined_at)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (channel_id, user_id) DO NOTHING
                    "#,
                    channel.id().uuid(),
                    user_id.uuid(),
                    channel.created_at(),
                )
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn update_channel_with_events(
        &self,
        channel: &Channel,
        events: &[OutboxRecord],
        voice_changes: &[VoiceChange],
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

        apply_voice_changes(&mut tx, channel.id(), voice_changes).await?;

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn apply_participant_joined(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError> {
        apply_voice_operation(
            &self.pool,
            channel_id,
            inbox_source,
            inbox_external_id,
            request_id,
            |channel| channel.join(user_id, session_id, occurred_at),
        )
        .await
    }

    async fn apply_participant_left(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        reason: LeaveReason,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError> {
        apply_voice_operation(
            &self.pool,
            channel_id,
            inbox_source,
            inbox_external_id,
            request_id,
            |channel| channel.leave(user_id, session_id, reason, occurred_at),
        )
        .await
    }

    async fn apply_participant_connection_aborted(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        reason: ConnectionAbortReason,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError> {
        apply_voice_operation(
            &self.pool,
            channel_id,
            inbox_source,
            inbox_external_id,
            request_id,
            |channel| channel.connection_aborted(user_id, session_id, reason, occurred_at),
        )
        .await
    }

    async fn apply_track_published(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        source: MediaSource,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError> {
        apply_voice_operation(
            &self.pool,
            channel_id,
            inbox_source,
            inbox_external_id,
            request_id,
            |channel| channel.publish_track(user_id, session_id, source, occurred_at),
        )
        .await
    }

    async fn apply_track_unpublished(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        source: MediaSource,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError> {
        apply_voice_operation(
            &self.pool,
            channel_id,
            inbox_source,
            inbox_external_id,
            request_id,
            |channel| channel.unpublish_track(user_id, session_id, source, occurred_at),
        )
        .await
    }

    async fn delete_channel_with_events(
        &self,
        channel_id: &ChannelId,
        events: &[OutboxRecord],
    ) -> Result<Vec<String>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"SELECT id FROM channels WHERE id = $1 FOR UPDATE"#,
            channel_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let storage_keys = sqlx::query_scalar!(
            r#"
            SELECT a.storage_key
            FROM attachments a
            INNER JOIN messages m ON m.id = a.message_id
            WHERE m.channel_id = $1
            "#,
            channel_id.uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
            INSERT INTO storage_cleanup_queue (storage_key)
            SELECT * FROM UNNEST($1::text[])
            ON CONFLICT (storage_key) DO NOTHING
            "#,
            &storage_keys,
        )
        .execute(&mut *tx)
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
        Ok(storage_keys)
    }

    async fn list_voice_channel_ids_by_server(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<ChannelId>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT id
            FROM channels
            WHERE server_id = $1 AND kind = 'voice'::kind
            "#,
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows.into_iter().map(|r| ChannelId::new(r.id)).collect())
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

    async fn list_channel_participants(
        &self,
        server_id: &ServerId,
        channel_id: &ChannelId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<User>, RepositoryError> {
        let kind_row = sqlx::query!(
            r#"
            SELECT kind AS "kind: ChannelKindDto"
            FROM channels
            WHERE id = $1 AND server_id = $2
            "#,
            channel_id.uuid(),
            server_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        let kind: ChannelKind = kind_row.kind.into();

        if kind.supports_voice() {
            let total = sqlx::query!(
                r#"
                SELECT COUNT(DISTINCT user_id) AS "count!"
                FROM channel_voice_participants
                WHERE channel_id = $1
                "#,
                channel_id.uuid(),
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
                INNER JOIN (
                    SELECT user_id, MIN(joined_at) AS first_joined_at
                    FROM channel_voice_participants
                    WHERE channel_id = $1
                    GROUP BY user_id
                ) p ON p.user_id = u.id
                ORDER BY p.first_joined_at ASC
                LIMIT $2 OFFSET $3
                "#,
                channel_id.uuid(),
                pagination.limit(),
                pagination.offset(),
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

            let items = users.into_iter().map(|dto| dto.into()).collect();
            return Ok(PaginatedResult::new(items, total, pagination));
        }

        if kind.supports_text() {
            let total = sqlx::query!(
                r#"
                SELECT COUNT(*) AS "count!"
                FROM channel_text_participants
                WHERE channel_id = $1
                "#,
                channel_id.uuid(),
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
                INNER JOIN channel_text_participants p ON p.user_id = u.id
                WHERE p.channel_id = $1
                ORDER BY p.joined_at ASC
                LIMIT $2 OFFSET $3
                "#,
                channel_id.uuid(),
                pagination.limit(),
                pagination.offset(),
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

            let items = users.into_iter().map(|dto| dto.into()).collect();
            return Ok(PaginatedResult::new(items, total, pagination));
        }

        Ok(PaginatedResult::new(Vec::new(), 0, pagination))
    }

    async fn join_text_with_events(
        &self,
        channel: &Channel,
        joined_user_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
            INSERT INTO channel_text_participants (channel_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            channel.id().uuid(),
            joined_user_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Ok(());
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn list_voice_presence_for_server(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<domain::channel::repository::VoicePresenceRow>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                cvp.channel_id,
                cvp.user_id,
                u.name AS user_name,
                bool_or(cvp.camera_on) AS "camera_on!",
                bool_or(cvp.screen_on) AS "screen_on!",
                EXISTS (
                    SELECT 1 FROM channel_muted_participants cmp
                    WHERE cmp.channel_id = cvp.channel_id
                      AND cmp.user_id = cvp.user_id
                ) AS "is_muted!"
            FROM channel_voice_participants cvp
            INNER JOIN channels c ON c.id = cvp.channel_id
            INNER JOIN users u ON u.id = cvp.user_id
            WHERE c.server_id = $1
            GROUP BY cvp.channel_id, cvp.user_id, u.name
            "#,
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| domain::channel::repository::VoicePresenceRow {
                channel_id: ChannelId::new(r.channel_id),
                user_id: UserId::new(r.user_id),
                user_name: r.user_name,
                is_muted: r.is_muted,
                camera_on: r.camera_on,
                screen_on: r.screen_on,
            })
            .collect())
    }

    async fn set_stream_viewer(
        &self,
        server_id: &ServerId,
        channel_id: &ChannelId,
        streamer_id: &UserId,
        viewer_id: &UserId,
        viewer_session_id: &SessionId,
        watching: bool,
        occurred_at: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<i64, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"SELECT 1 AS one FROM channels WHERE id = $1 AND server_id = $2 FOR UPDATE"#,
            channel_id.uuid(),
            server_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;
        let server_id = server_id.uuid();

        if watching {
            sqlx::query!(
                r#"
                INSERT INTO channel_stream_viewers
                    (channel_id, streamer_id, viewer_id, viewer_session_id, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (channel_id, streamer_id, viewer_id, viewer_session_id)
                DO UPDATE SET updated_at = EXCLUDED.updated_at
                "#,
                channel_id.uuid(),
                streamer_id.uuid(),
                viewer_id.uuid(),
                viewer_session_id.as_str(),
                occurred_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        } else {
            sqlx::query!(
                r#"
                DELETE FROM channel_stream_viewers
                WHERE channel_id = $1
                  AND streamer_id = $2
                  AND viewer_id = $3
                  AND viewer_session_id = $4
                "#,
                channel_id.uuid(),
                streamer_id.uuid(),
                viewer_id.uuid(),
                viewer_session_id.as_str(),
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        let count =
            count_stream_viewers(&mut tx, channel_id.uuid(), streamer_id.uuid()).await?;

        let record = stream_viewers_record(
            channel_id.uuid(),
            server_id,
            streamer_id.uuid(),
            count,
            occurred_at,
            request_id,
        );
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(count)
    }

    async fn list_stream_viewer_counts_for_server(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<domain::channel::repository::StreamViewerCountRow>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                v.channel_id,
                v.streamer_id,
                count(DISTINCT v.viewer_id) AS "count!"
            FROM channel_stream_viewers v
            INNER JOIN channels c ON c.id = v.channel_id
            WHERE c.server_id = $1
              AND v.viewer_id <> v.streamer_id
              AND EXISTS (
                SELECT 1 FROM channel_voice_participants p
                WHERE p.channel_id = v.channel_id
                  AND p.user_id = v.viewer_id
                  AND p.session_id = v.viewer_session_id
              )
              AND EXISTS (
                SELECT 1 FROM channel_voice_participants s
                WHERE s.channel_id = v.channel_id
                  AND s.user_id = v.streamer_id
                  AND s.screen_on = true
              )
            GROUP BY v.channel_id, v.streamer_id
            "#,
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| domain::channel::repository::StreamViewerCountRow {
                channel_id: ChannelId::new(r.channel_id),
                streamer_id: UserId::new(r.streamer_id),
                count: r.count,
            })
            .collect())
    }

    async fn is_text_channel_member(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM channel_text_participants
                WHERE channel_id = $1 AND user_id = $2
            ) AS "exists!"
            "#,
            channel_id.uuid(),
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.exists)
    }

    async fn list_text_channel_member_ids(
        &self,
        channel_id: &ChannelId,
    ) -> Result<Vec<UserId>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT user_id
            FROM channel_text_participants
            WHERE channel_id = $1
            "#,
            channel_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows.into_iter().map(|r| UserId::new(r.user_id)).collect())
    }

    async fn find_channel_by_id(&self, id: &ChannelId) -> Result<Channel, RepositoryError> {
        let dto = sqlx::query_as!(
            ChannelDto,
            r#"
            SELECT id, server_id, owner_id, name,
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
        let (muted_participants, voice_participants, camera_participants, screen_participants) =
            if kind.supports_voice() {
            let map: HashMap<UserId, ParticipantMute> = muted_dtos
                .into_iter()
                .map(|dto| {
                    let mute: ParticipantMute = dto.into();
                    (mute.user_id, mute)
                })
                .collect();

            let voice_rows = sqlx::query!(
                r#"
                SELECT user_id, session_id, camera_on, screen_on
                FROM channel_voice_participants
                WHERE channel_id = $1
                "#,
                id.uuid(),
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

            let mut voice: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
            let mut camera: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
            let mut screen: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
            for r in voice_rows {
                let user_id = UserId::new(r.user_id);
                let session_id = SessionId::new(r.session_id);
                if r.camera_on {
                    camera.entry(user_id).or_default().insert(session_id.clone());
                }
                if r.screen_on {
                    screen.entry(user_id).or_default().insert(session_id.clone());
                }
                voice.entry(user_id).or_default().insert(session_id);
            }

            (Some(map), Some(voice), Some(camera), Some(screen))
        } else {
            (None, None, None, None)
        };

        let text_participants = if kind.supports_text() {
            Some(HashSet::<UserId>::new())
        } else {
            None
        };

        let channel = Channel::restore(
            ChannelId::new(dto.id),
            ServerId::new(dto.server_id),
            UserId::new(dto.owner_id),
            ChannelName::new(dto.name),
            kind,
            dto.visibility.into(),
            dto.password_hash.map(PasswordHash::new),
            dto.created_at,
            dto.updated_at,
            muted_participants,
            voice_participants,
            camera_participants,
            screen_participants,
            text_participants,
            dto.last_join,
        );

        Ok(channel)
    }
}

async fn apply_voice_changes(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: &ChannelId,
    changes: &[VoiceChange],
) -> Result<(), RepositoryError> {
    for change in changes {
        match change {
            VoiceChange::Added {
                user_id,
                session_id,
            } => {
                sqlx::query!(
                    r#"
                    INSERT INTO channel_voice_participants (channel_id, user_id, session_id)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                    channel_id.uuid(),
                    user_id.uuid(),
                    session_id.as_str(),
                )
                .execute(&mut **tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
            VoiceChange::Removed {
                user_id,
                session_id,
            } => {
                sqlx::query!(
                    r#"
                    DELETE FROM channel_voice_participants
                    WHERE channel_id = $1 AND user_id = $2 AND session_id = $3
                    "#,
                    channel_id.uuid(),
                    user_id.uuid(),
                    session_id.as_str(),
                )
                .execute(&mut **tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
        }
    }
    Ok(())
}

async fn apply_media_changes(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: &ChannelId,
    changes: &[MediaChange],
) -> Result<(), RepositoryError> {
    for change in changes {
        match change.source {
            MediaSource::Camera => {
                sqlx::query!(
                    r#"
                    UPDATE channel_voice_participants
                    SET camera_on = $4
                    WHERE channel_id = $1 AND user_id = $2 AND session_id = $3
                    "#,
                    channel_id.uuid(),
                    change.user_id.uuid(),
                    change.session_id.as_str(),
                    change.enabled,
                )
                .execute(&mut **tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
            MediaSource::Screen => {
                sqlx::query!(
                    r#"
                    UPDATE channel_voice_participants
                    SET screen_on = $4
                    WHERE channel_id = $1 AND user_id = $2 AND session_id = $3
                    "#,
                    channel_id.uuid(),
                    change.user_id.uuid(),
                    change.session_id.as_str(),
                    change.enabled,
                )
                .execute(&mut **tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
        }
    }
    Ok(())
}

pub(crate) async fn count_stream_viewers(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: uuid::Uuid,
    streamer_id: uuid::Uuid,
) -> Result<i64, RepositoryError> {
    let row = sqlx::query!(
        r#"
        SELECT count(DISTINCT v.viewer_id) AS "count!"
        FROM channel_stream_viewers v
        WHERE v.channel_id = $1
          AND v.streamer_id = $2
          AND v.viewer_id <> v.streamer_id
          AND EXISTS (
            SELECT 1 FROM channel_voice_participants p
            WHERE p.channel_id = v.channel_id
              AND p.user_id = v.viewer_id
              AND p.session_id = v.viewer_session_id
          )
          AND EXISTS (
            SELECT 1 FROM channel_voice_participants s
            WHERE s.channel_id = v.channel_id
              AND s.user_id = v.streamer_id
              AND s.screen_on = true
          )
        "#,
        channel_id,
        streamer_id,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(row.count)
}

pub(crate) fn stream_viewers_record(
    channel_id: uuid::Uuid,
    server_id: uuid::Uuid,
    streamer_id: uuid::Uuid,
    count: i64,
    occurred_at: DateTime<Utc>,
    request_id: Option<String>,
) -> OutboxRecord {
    OutboxRecord {
        id: uuid::Uuid::new_v4(),
        aggregate_id: channel_id,
        aggregate_type: domain::shared::outbox::outbox_record::AggregateType::Channel,
        parent_id: Some(server_id),
        parent_type: Some(domain::shared::outbox::outbox_record::AggregateType::Server),
        event_type: "stream.viewers".to_string(),
        payload: serde_json::json!({
            "channel_id": channel_id,
            "server_id": server_id,
            "streamer_id": streamer_id,
            "count": count,
            "occurred_at": occurred_at,
        }),
        occurred_at,
        request_id,
    }
}

async fn clear_stream_for_streamer(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: uuid::Uuid,
    server_id: uuid::Uuid,
    streamer_id: uuid::Uuid,
    occurred_at: DateTime<Utc>,
    request_id: Option<String>,
) -> Result<Option<OutboxRecord>, RepositoryError> {
    let deleted = sqlx::query!(
        r#"
        DELETE FROM channel_stream_viewers
        WHERE channel_id = $1 AND streamer_id = $2
        "#,
        channel_id,
        streamer_id,
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    if deleted.rows_affected() == 0 {
        return Ok(None);
    }

    Ok(Some(stream_viewers_record(
        channel_id,
        server_id,
        streamer_id,
        0,
        occurred_at,
        request_id,
    )))
}

async fn recompute_session_watched_streams(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: uuid::Uuid,
    server_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    viewer_session_id: &str,
    occurred_at: DateTime<Utc>,
    request_id: Option<String>,
) -> Result<Vec<OutboxRecord>, RepositoryError> {
    let watched = sqlx::query!(
        r#"
        SELECT DISTINCT streamer_id
        FROM channel_stream_viewers
        WHERE channel_id = $1 AND viewer_id = $2 AND viewer_session_id = $3
        "#,
        channel_id,
        viewer_id,
        viewer_session_id,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    let mut records = Vec::new();
    for w in watched {
        let count = count_stream_viewers(tx, channel_id, w.streamer_id).await?;
        records.push(stream_viewers_record(
            channel_id,
            server_id,
            w.streamer_id,
            count,
            occurred_at,
            request_id.clone(),
        ));
    }
    Ok(records)
}

async fn cleanup_session_stream_viewer(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: uuid::Uuid,
    server_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    viewer_session_id: &str,
    occurred_at: DateTime<Utc>,
    request_id: Option<String>,
) -> Result<Vec<OutboxRecord>, RepositoryError> {
    let watched = sqlx::query!(
        r#"
        SELECT DISTINCT streamer_id
        FROM channel_stream_viewers
        WHERE channel_id = $1 AND viewer_id = $2 AND viewer_session_id = $3
        "#,
        channel_id,
        viewer_id,
        viewer_session_id,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    let deleted = sqlx::query!(
        r#"
        DELETE FROM channel_stream_viewers
        WHERE channel_id = $1 AND viewer_id = $2 AND viewer_session_id = $3
        "#,
        channel_id,
        viewer_id,
        viewer_session_id,
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    if deleted.rows_affected() == 0 {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    for w in watched {
        let count = count_stream_viewers(tx, channel_id, w.streamer_id).await?;
        records.push(stream_viewers_record(
            channel_id,
            server_id,
            w.streamer_id,
            count,
            occurred_at,
            request_id.clone(),
        ));
    }
    Ok(records)
}

async fn apply_voice_operation<F>(
    pool: &Pool<Postgres>,
    channel_id: &ChannelId,
    inbox_source: &str,
    inbox_external_id: &str,
    request_id: Option<String>,
    mutate: F,
) -> Result<bool, ChannelOperationError>
where
    F: FnOnce(&mut Channel) -> Result<(), domain::channel::ChannelError>,
{
    let mut tx = pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;

    let inbox_result = sqlx::query!(
        r#"
        INSERT INTO webhook_inbox (source, external_id)
        VALUES (($1::text)::webhook_source, $2)
        ON CONFLICT DO NOTHING
        "#,
        inbox_source,
        inbox_external_id,
    )
    .execute(&mut *tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    if inbox_result.rows_affected() == 0 {
        return Ok(false);
    }

    let mut channel = load_channel_for_update(&mut tx, channel_id).await?;

    mutate(&mut channel)?;

    let channel_events = channel.pull_events();
    let departed: Vec<uuid::Uuid> = channel_events
        .iter()
        .filter_map(|e| e.departed_user().map(|u| u.uuid()))
        .collect();
    let screen_stopped: Vec<uuid::Uuid> = channel_events
        .iter()
        .filter_map(|e| e.screen_stopped_user().map(|u| u.uuid()))
        .collect();
    let screen_started: Vec<uuid::Uuid> = channel_events
        .iter()
        .filter_map(|e| e.screen_started_user().map(|u| u.uuid()))
        .collect();
    let mut events: Vec<OutboxRecord> = channel_events
        .iter()
        .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
        .collect::<Result<_, _>>()
        .map_err(|e| RepositoryError::Internal(format!("outbox error: {e}")))?;
    let voice_changes = channel.pull_voice_changes();
    let media_changes = channel.pull_media_changes();
    let server_id = channel.server_id().uuid();
    let channel_uuid = channel.id().uuid();

    sqlx::query!(
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

    let removed_sessions: Vec<(uuid::Uuid, String)> = voice_changes
        .iter()
        .filter_map(|vc| match vc {
            VoiceChange::Removed {
                user_id,
                session_id,
            } => Some((user_id.uuid(), session_id.as_str().to_string())),
            VoiceChange::Added { .. } => None,
        })
        .collect();
    let added_sessions: Vec<(uuid::Uuid, String)> = voice_changes
        .iter()
        .filter_map(|vc| match vc {
            VoiceChange::Added {
                user_id,
                session_id,
            } => Some((user_id.uuid(), session_id.as_str().to_string())),
            VoiceChange::Removed { .. } => None,
        })
        .collect();

    apply_voice_changes(&mut tx, channel.id(), &voice_changes).await?;
    apply_media_changes(&mut tx, channel.id(), &media_changes).await?;

    for (viewer, session) in added_sessions {
        let extra = recompute_session_watched_streams(
            &mut tx,
            channel_uuid,
            server_id,
            viewer,
            &session,
            Utc::now(),
            request_id.clone(),
        )
        .await?;
        events.extend(extra);
    }

    for (viewer, session) in removed_sessions {
        let extra = cleanup_session_stream_viewer(
            &mut tx,
            channel_uuid,
            server_id,
            viewer,
            &session,
            Utc::now(),
            request_id.clone(),
        )
        .await?;
        events.extend(extra);
    }

    for streamer in departed {
        if let Some(record) = clear_stream_for_streamer(
            &mut tx,
            channel_uuid,
            server_id,
            streamer,
            Utc::now(),
            request_id.clone(),
        )
        .await?
        {
            events.push(record);
        }
    }

    for streamer in screen_stopped {
        if let Some(record) = clear_stream_for_streamer(
            &mut tx,
            channel_uuid,
            server_id,
            streamer,
            Utc::now(),
            request_id.clone(),
        )
        .await?
        {
            events.push(record);
        }
    }

    for streamer in screen_started {
        let count = count_stream_viewers(&mut tx, channel_uuid, streamer).await?;
        events.push(stream_viewers_record(
            channel_uuid,
            server_id,
            streamer,
            count,
            Utc::now(),
            request_id.clone(),
        ));
    }

    save_outbox_records(&mut tx, &events).await?;

    tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
    Ok(true)
}

async fn load_channel_for_update(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: &ChannelId,
) -> Result<Channel, RepositoryError> {
    let dto = sqlx::query_as!(
        ChannelDto,
        r#"
        SELECT id, server_id, owner_id, name,
               kind AS "kind: ChannelKindDto",
               visibility AS "visibility: ChannelVisibilityDto",
               password_hash, created_at, updated_at, last_join
        FROM channels
        WHERE id = $1
        FOR UPDATE
        "#,
        channel_id.uuid(),
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .ok_or(RepositoryError::NotFound)?;

    let kind: ChannelKind = dto.kind.into();

    let muted_dtos = sqlx::query_as!(
        MutedParticipantDto,
        r#"
        SELECT channel_id, user_id, muted_by, muted_at
        FROM channel_muted_participants
        WHERE channel_id = $1
        "#,
        channel_id.uuid(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    let (muted_participants, voice_participants, camera_participants, screen_participants) =
        if kind.supports_voice() {
        let map: HashMap<UserId, ParticipantMute> = muted_dtos
            .into_iter()
            .map(|d| {
                let mute: ParticipantMute = d.into();
                (mute.user_id, mute)
            })
            .collect();

        let voice_rows = sqlx::query!(
            r#"
            SELECT user_id, session_id, camera_on, screen_on
            FROM channel_voice_participants
            WHERE channel_id = $1
            "#,
            channel_id.uuid(),
        )
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let mut voice: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
        let mut camera: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
        let mut screen: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
        for r in voice_rows {
            let user_id = UserId::new(r.user_id);
            let session_id = SessionId::new(r.session_id);
            if r.camera_on {
                camera.entry(user_id).or_default().insert(session_id.clone());
            }
            if r.screen_on {
                screen.entry(user_id).or_default().insert(session_id.clone());
            }
            voice.entry(user_id).or_default().insert(session_id);
        }

        (Some(map), Some(voice), Some(camera), Some(screen))
    } else {
        (None, None, None, None)
    };

    let text_participants = if kind.supports_text() {
        Some(HashSet::<UserId>::new())
    } else {
        None
    };

    Ok(Channel::restore(
        ChannelId::new(dto.id),
        ServerId::new(dto.server_id),
        UserId::new(dto.owner_id),
        ChannelName::new(dto.name),
        kind,
        dto.visibility.into(),
        dto.password_hash.map(PasswordHash::new),
        dto.created_at,
        dto.updated_at,
        muted_participants,
        voice_participants,
        camera_participants,
        screen_participants,
        text_participants,
        dto.last_join,
    ))
}
