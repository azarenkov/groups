use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use messaging_domain::{channel::Channel, channel::MediaChange, channel::VoiceChange, channel::events::participant::MediaSource, channel::id::ChannelId, channel::kind::ChannelKind, channel::name::ChannelName, channel::participant_mute::ParticipantMute, channel::repository::ChannelRepository, channel::repository::FriendVoicePresenceRow, channel::repository::RecentVoiceRoomRow, channel::repository::VoiceParticipantRow, channel::session_id::SessionId, server::id::ServerId};
use domain::{pagination::PaginatedResult, pagination::Pagination, password_hash::PasswordHash, user::id::UserId, user::name::UserName};
use ports::{outbox::record::OutboxRecord, repository::RepositoryError};
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

    async fn list_voice_channel_ids_with_participant(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<ChannelId>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT channel_id
            FROM channel_voice_participants
            WHERE user_id = $1
            "#,
            user_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows.into_iter().map(|r| ChannelId::new(r.channel_id)).collect())
    }

    async fn list_channel_participants(
        &self,
        server_id: &ServerId,
        channel_id: &ChannelId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<(UserId, UserName, String, Option<DateTime<Utc>>)>, RepositoryError> {
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

            let users = sqlx::query!(
                r#"
                SELECT u.user_id, u.name, CASE WHEN u.active THEN u.bio ELSE '' END AS "bio!", u.profile_updated_at
                FROM messaging_user_profiles u
                INNER JOIN (
                    SELECT user_id, MIN(joined_at) AS first_joined_at
                    FROM channel_voice_participants
                    WHERE channel_id = $1
                    GROUP BY user_id
                ) p ON p.user_id = u.user_id
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

            let items = users
                .into_iter()
                .map(|r| (UserId::new(r.user_id), UserName::new(r.name), r.bio, r.profile_updated_at))
                .collect();
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

            let users = sqlx::query!(
                r#"
                SELECT u.user_id, u.name, CASE WHEN u.active THEN u.bio ELSE '' END AS "bio!", u.profile_updated_at
                FROM messaging_user_profiles u
                INNER JOIN channel_text_participants p ON p.user_id = u.user_id
                WHERE p.channel_id = $1
                ORDER BY p.joined_at ASC, u.user_id ASC
                LIMIT $2 OFFSET $3
                "#,
                channel_id.uuid(),
                pagination.limit(),
                pagination.offset(),
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

            let items = users
                .into_iter()
                .map(|r| (UserId::new(r.user_id), UserName::new(r.name), r.bio, r.profile_updated_at))
                .collect();
            return Ok(PaginatedResult::new(items, total, pagination));
        }

        Ok(PaginatedResult::new(Vec::new(), 0, pagination))
    }

    async fn list_voice_presence_for_server(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<messaging_domain::channel::repository::VoicePresenceRow>, RepositoryError> {
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
                ) AS "is_muted!",
                (
                    SELECT cmp.muted_by FROM channel_muted_participants cmp
                    WHERE cmp.channel_id = cvp.channel_id
                      AND cmp.user_id = cvp.user_id
                ) AS "muted_by?"
            FROM channel_voice_participants cvp
            INNER JOIN channels c ON c.id = cvp.channel_id
            INNER JOIN messaging_user_profiles u ON u.user_id = cvp.user_id
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
            .map(|r| messaging_domain::channel::repository::VoicePresenceRow {
                channel_id: ChannelId::new(r.channel_id),
                user_id: UserId::new(r.user_id),
                user_name: r.user_name,
                is_muted: r.is_muted,
                muted_by: r.muted_by.map(UserId::new),
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
    ) -> Result<Vec<messaging_domain::channel::repository::StreamViewerCountRow>, RepositoryError> {
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
            .map(|r| messaging_domain::channel::repository::StreamViewerCountRow {
                channel_id: ChannelId::new(r.channel_id),
                streamer_id: UserId::new(r.streamer_id),
                count: r.count,
            })
            .collect())
    }

    async fn list_voice_participants_joined_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<VoiceParticipantRow>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT channel_id, user_id, session_id, camera_on, screen_on
            FROM channel_voice_participants
            WHERE joined_at < $1
            "#,
            cutoff,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| VoiceParticipantRow {
                channel_id: ChannelId::new(r.channel_id),
                user_id: UserId::new(r.user_id),
                session_id: r.session_id,
                camera_on: r.camera_on,
                screen_on: r.screen_on,
            })
            .collect())
    }

    async fn list_friends_voice_presence(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<FriendVoicePresenceRow>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                cvp.user_id,
                u.name AS user_name,
                c.id AS channel_id,
                c.name AS channel_name,
                s.id AS server_id,
                s.name AS server_name,
                MIN(cvp.joined_at) AS "joined_at!",
                bool_or(cvp.camera_on) AS "camera_on!",
                bool_or(cvp.screen_on) AS "screen_on!",
                EXISTS (
                    SELECT 1 FROM channel_muted_participants cmp
                    WHERE cmp.channel_id = c.id
                      AND cmp.user_id = cvp.user_id
                ) AS "is_muted!"
            FROM channel_voice_participants cvp
            INNER JOIN channels c ON c.id = cvp.channel_id
            INNER JOIN servers s ON s.id = c.server_id
            INNER JOIN server_members me ON me.server_id = s.id AND me.user_id = $1
            INNER JOIN friendships f
                ON f.user_id = $1 AND f.friend_id = cvp.user_id AND f.status = 'accepted'
            INNER JOIN messaging_user_profiles u ON u.user_id = cvp.user_id AND u.show_activity
            GROUP BY cvp.user_id, u.name, c.id, c.name, s.id, s.name
            ORDER BY MIN(cvp.joined_at) DESC
            "#,
            user_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| FriendVoicePresenceRow {
                user_id: UserId::new(r.user_id),
                user_name: r.user_name,
                channel_id: ChannelId::new(r.channel_id),
                channel_name: r.channel_name,
                server_id: ServerId::new(r.server_id),
                server_name: r.server_name,
                joined_at: r.joined_at,
                is_muted: r.is_muted,
                camera_on: r.camera_on,
                screen_on: r.screen_on,
            })
            .collect())
    }

    async fn list_recent_voice_rooms(
        &self,
        user_id: &UserId,
        limit: i64,
    ) -> Result<Vec<RecentVoiceRoomRow>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                c.id AS channel_id,
                c.name AS channel_name,
                s.id AS server_id,
                s.name AS server_name,
                cvv.last_joined_at AS last_join,
                (
                    SELECT COUNT(DISTINCT v.user_id)
                    FROM channel_voice_participants v
                    WHERE v.channel_id = c.id
                ) AS "live_count!"
            FROM channel_voice_visits cvv
            INNER JOIN channels c ON c.id = cvv.channel_id
            INNER JOIN servers s ON s.id = c.server_id
            INNER JOIN server_members me ON me.server_id = s.id AND me.user_id = cvv.user_id
            WHERE cvv.user_id = $1
              AND c.kind = 'voice'
              AND cvv.last_joined_at > now() - interval '7 days'
            ORDER BY cvv.last_joined_at DESC
            LIMIT $2
            "#,
            user_id.uuid(),
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| RecentVoiceRoomRow {
                channel_id: ChannelId::new(r.channel_id),
                channel_name: r.channel_name,
                server_id: ServerId::new(r.server_id),
                server_name: r.server_name,
                live_count: r.live_count,
                last_join: r.last_join,
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

    async fn text_members_by_names(
        &self,
        channel_id: &ChannelId,
        lowercase_names: &[String],
    ) -> Result<Vec<(UserId, String)>, RepositoryError> {
        if lowercase_names.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query!(
            r#"
            SELECT p.user_id, lower(up.name) AS "name!"
            FROM channel_text_participants p
            INNER JOIN messaging_user_profiles up ON up.user_id = p.user_id
            WHERE p.channel_id = $1 AND lower(up.name) = ANY($2)
            "#,
            channel_id.uuid(),
            lowercase_names,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| (UserId::new(r.user_id), r.name))
            .collect())
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
            SELECT channel_id, user_id, muted_by, muted_at, self_muted
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

pub(crate) async fn apply_voice_changes(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    channel_id: &ChannelId,
    changes: &[VoiceChange],
) -> Result<(), RepositoryError> {
    for change in changes {
        match change {
            VoiceChange::Added {
                user_id,
                session_id,
                joined_at,
            } => {
                sqlx::query!(
                    r#"
                    INSERT INTO channel_voice_participants (channel_id, user_id, session_id, joined_at)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT DO NOTHING
                    "#,
                    channel_id.uuid(),
                    user_id.uuid(),
                    session_id.as_str(),
                    joined_at,
                )
                .execute(&mut **tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                sqlx::query!(
                    r#"
                    INSERT INTO channel_voice_visits (channel_id, user_id, last_joined_at)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (channel_id, user_id) DO UPDATE
                    SET last_joined_at = GREATEST(channel_voice_visits.last_joined_at, EXCLUDED.last_joined_at)
                    "#,
                    channel_id.uuid(),
                    user_id.uuid(),
                    joined_at,
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

pub(crate) async fn apply_media_changes(
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
        aggregate_type: ports::outbox::record::AggregateType::Channel,
        parent_id: Some(server_id),
        parent_type: Some(ports::outbox::record::AggregateType::Server),
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

pub(crate) async fn clear_stream_for_streamer(
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

pub(crate) async fn recompute_session_watched_streams(
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

pub(crate) async fn cleanup_session_stream_viewer(
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


pub(crate) async fn load_channel_for_update(
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
        SELECT channel_id, user_id, muted_by, muted_at, self_muted
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
