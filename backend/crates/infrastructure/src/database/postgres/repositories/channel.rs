use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use domain::{
    channel::{
        Channel, VoiceChange,
        events::participant::{ConnectionAbortReason, LeaveReason},
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
                SELECT u.id, u.name, u.email, u.password_hash, u.created_at, u.updated_at, u.email_verified_at, u.phone_verified_at
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
                SELECT u.id, u.name, u.email, u.password_hash, u.created_at, u.updated_at, u.email_verified_at, u.phone_verified_at
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
            SELECT DISTINCT
                cvp.channel_id,
                cvp.user_id,
                u.name AS user_name,
                EXISTS (
                    SELECT 1 FROM channel_muted_participants cmp
                    WHERE cmp.channel_id = cvp.channel_id
                      AND cmp.user_id = cvp.user_id
                ) AS "is_muted!"
            FROM channel_voice_participants cvp
            INNER JOIN channels c ON c.id = cvp.channel_id
            INNER JOIN users u ON u.id = cvp.user_id
            WHERE c.server_id = $1
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
        let (muted_participants, voice_participants) = if kind.supports_voice() {
            let map: HashMap<UserId, ParticipantMute> = muted_dtos
                .into_iter()
                .map(|dto| {
                    let mute: ParticipantMute = dto.into();
                    (mute.user_id, mute)
                })
                .collect();

            let voice_rows = sqlx::query!(
                r#"
                SELECT user_id, session_id
                FROM channel_voice_participants
                WHERE channel_id = $1
                "#,
                id.uuid(),
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

            let mut voice: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
            for r in voice_rows {
                voice
                    .entry(UserId::new(r.user_id))
                    .or_default()
                    .insert(SessionId::new(r.session_id));
            }

            (Some(map), Some(voice))
        } else {
            (None, None)
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

    let events: Vec<OutboxRecord> = channel
        .pull_events()
        .iter()
        .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
        .collect::<Result<_, _>>()
        .map_err(|e| RepositoryError::Internal(format!("outbox error: {e}")))?;
    let voice_changes = channel.pull_voice_changes();

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

    apply_voice_changes(&mut tx, channel.id(), &voice_changes).await?;

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

    let (muted_participants, voice_participants) = if kind.supports_voice() {
        let map: HashMap<UserId, ParticipantMute> = muted_dtos
            .into_iter()
            .map(|d| {
                let mute: ParticipantMute = d.into();
                (mute.user_id, mute)
            })
            .collect();

        let voice_rows = sqlx::query!(
            r#"
            SELECT user_id, session_id
            FROM channel_voice_participants
            WHERE channel_id = $1
            "#,
            channel_id.uuid(),
        )
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let mut voice: HashMap<UserId, HashSet<SessionId>> = HashMap::new();
        for r in voice_rows {
            voice
                .entry(UserId::new(r.user_id))
                .or_default()
                .insert(SessionId::new(r.session_id));
        }

        (Some(map), Some(voice))
    } else {
        (None, None)
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
        text_participants,
        dto.last_join,
    ))
}
