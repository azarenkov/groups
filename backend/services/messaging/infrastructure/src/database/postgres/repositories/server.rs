use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

use async_trait::async_trait;

use domain::{pagination::PaginatedResult, pagination::Pagination, password_hash::PasswordHash, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;
use messaging_domain::{channel::Channel, channel::id::ChannelId, channel::kind::ChannelKind, channel::name::ChannelName, channel::participant_deafen::ParticipantDeafen, channel::participant_mute::ParticipantMute, channel::session_id::SessionId, server::Server, server::id::ServerId, server::invite_link::InviteLink, server::invite_link::InviteToken, server::repository::ServerRepository, server::repository::MutualServer};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    dto::{
        channel::ChannelDto, kind::ChannelKindDto, muted_participant::MutedParticipantDto,
        server::ServerDto, visibility::ChannelVisibilityDto,
    },
    error::map_sqlx_error_to_domain_error,
    repositories::{invite_link::INVITE_LINK_COLUMNS, invite_link::InviteLinkRow},
};

pub struct PostgresServerRepository {
    pool: Pool<Postgres>,
}

impl PostgresServerRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn has_user_messaging_state(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<bool, RepositoryError> {
        let row: (bool,) = sqlx::query_as(
            r#"
            SELECT
                EXISTS(SELECT 1 FROM servers WHERE owner_id = $1)
                OR EXISTS(SELECT 1 FROM server_members WHERE user_id = $1)
                OR EXISTS(SELECT 1 FROM friendships WHERE user_id = $1)
                OR EXISTS(SELECT 1 FROM messages WHERE author_id = $1)
                OR EXISTS(SELECT 1 FROM dm_messages WHERE sender_id = $1)
                OR EXISTS(SELECT 1 FROM dm_messages WHERE recipient_id = $1)
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.0)
    }

    pub async fn activity_audience(
        &self,
        user_id: uuid::Uuid,
        candidates: &[uuid::Uuid],
    ) -> Result<Vec<uuid::Uuid>, RepositoryError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT related.other
            FROM (
                SELECT f.friend_id AS other
                FROM friendships f
                WHERE f.user_id = $1 AND f.status = 'accepted'
                UNION
                SELECT m2.user_id AS other
                FROM server_members m1
                INNER JOIN server_members m2 ON m2.server_id = m1.server_id
                WHERE m1.user_id = $1
            ) related
            WHERE related.other = ANY($2) AND related.other <> $1
            "#,
        )
        .bind(user_id)
        .bind(candidates)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
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
    ) -> Result<PaginatedResult<(UserId, UserName)>, RepositoryError> {
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

        let users = sqlx::query!(
            r#"
                SELECT u.user_id, u.name
                FROM messaging_user_profiles u
                INNER JOIN server_members sm ON sm.user_id = u.user_id
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

        let items = users
            .into_iter()
            .map(|r| (UserId::new(r.user_id), UserName::new(r.name)))
            .collect();
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
                password_hash, created_at, updated_at, last_join, is_default
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
            SELECT channel_id, user_id, muted_by, muted_at, self_muted
            FROM channel_muted_participants
            WHERE channel_id = ANY($1)
            "#,
            &channel_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let deafened_rows: Vec<(Uuid, Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT channel_id, user_id, deafened_by, deafened_at FROM channel_deafened_participants WHERE channel_id = ANY($1)",
        )
        .bind(&channel_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let mut deafened_by_channel: HashMap<Uuid, HashMap<UserId, ParticipantDeafen>> = HashMap::new();
        for (channel_id, user_id, deafened_by, deafened_at) in deafened_rows {
            let user_id = UserId::new(user_id);
            deafened_by_channel
                .entry(channel_id)
                .or_default()
                .insert(user_id, ParticipantDeafen::new(user_id, UserId::new(deafened_by), deafened_at));
        }

        let mut muted_by_channel: HashMap<Uuid, HashMap<UserId, ParticipantMute>> = HashMap::new();
        for dto in muted_dtos {
            let user_id = UserId::new(dto.user_id);
            let mute = ParticipantMute::new(
                user_id,
                UserId::new(dto.muted_by),
                dto.muted_at,
                dto.self_muted,
            );
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
                let (muted, deafened, voice, camera, screen) = if kind.supports_voice() {
                    (
                        Some(muted_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(deafened_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(voice_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(camera_by_channel.remove(&dto.id).unwrap_or_default()),
                        Some(screen_by_channel.remove(&dto.id).unwrap_or_default()),
                    )
                } else {
                    (None, None, None, None, None)
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
                    deafened,
                    voice,
                    camera,
                    screen,
                    text,
                    dto.last_join,
                )
                .with_default(dto.is_default)
            })
            .collect();

        Ok(PaginatedResult::new(items, total, pagination))
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

    async fn find_mutual_servers(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Vec<MutualServer>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT s.id, s.name, s.accent_color
            FROM servers s
            INNER JOIN server_members a ON a.server_id = s.id AND a.user_id = $1
            INNER JOIN server_members b ON b.server_id = s.id AND b.user_id = $2
            ORDER BY s.name ASC
            "#,
            user_a.uuid(),
            user_b.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| MutualServer {
                server_id: ServerId::new(r.id),
                name: r.name,
                accent_color: r.accent_color,
            })
            .collect())
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

    async fn list_invite_links(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<InviteLink>, RepositoryError> {
        let rows: Vec<InviteLinkRow> = sqlx::query_as(&format!(
            "SELECT {INVITE_LINK_COLUMNS} FROM server_invite_links l LEFT JOIN messaging_user_profiles u ON u.user_id = l.created_by WHERE l.server_id = $1 AND l.revoked_at IS NULL AND (l.expires_at IS NULL OR l.expires_at > now()) AND (l.max_uses IS NULL OR l.uses < l.max_uses) ORDER BY l.is_primary DESC, l.created_at DESC"
        ))
        .bind(server_id.uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(InviteLink::from).collect())
    }

    async fn find_invite_link(
        &self,
        token: &InviteToken,
    ) -> Result<Option<InviteLink>, RepositoryError> {
        let row: Option<InviteLinkRow> = sqlx::query_as(&format!(
            "SELECT {INVITE_LINK_COLUMNS} FROM server_invite_links l LEFT JOIN messaging_user_profiles u ON u.user_id = l.created_by WHERE l.token = $1"
        ))
        .bind(token.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(InviteLink::from))
    }

    async fn count_members(&self, server_id: &ServerId) -> Result<i64, RepositoryError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_members WHERE server_id = $1")
            .bind(server_id.uuid())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(count)
    }


}
