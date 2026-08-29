use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use messaging_domain::{events::MessagingEvent, channel::Channel, channel::id::ChannelId, channel::participant_mute::ParticipantMute, dm_message::DmMessage, dm_message::id::DmMessageId, message::id::MessageId, server::Server, server::description::ServerDescription, server::name::ServerName, server::id::ServerId, server::invite_code::InviteCode, server::role::ServerRole, server::unit_of_work::ServerTransaction, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::{
    dto::{kind::ChannelKindDto, visibility::ChannelVisibilityDto},
    error::map_sqlx_error_to_domain_error,
    repositories::{
        channel::{count_stream_viewers, load_channel_for_update, stream_viewers_record},
        dm_message::insert_dm_message,
        outbox_record::save_outbox_records,
    },
};

pub struct PostgresServerUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresServerUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresServerTransaction {
    tx: Transaction<'static, Postgres>,
    server_id: ServerId,
}

pub async fn lock_server(
    tx: &mut Transaction<'_, Postgres>,
    server_id: &ServerId,
) -> Result<(), RepositoryError> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::text, 42))",
        server_id.uuid().to_string(),
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(())
}

pub async fn load_server(
    tx: &mut Transaction<'_, Postgres>,
    server_id: &ServerId,
) -> Result<Server, RepositoryError> {
    let row = sqlx::query!(
        r#"
        SELECT id, name, owner_id, description, accent_color, created_at, updated_at
        FROM servers
        WHERE id = $1
        "#,
        server_id.uuid(),
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .ok_or(RepositoryError::NotFound)?;
    let roles = sqlx::query!(
        r#"
        SELECT id, server_id, name, color, permissions, position, created_at
        FROM server_roles
        WHERE server_id = $1
        ORDER BY position DESC, created_at ASC
        "#,
        server_id.uuid(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .into_iter()
    .map(|r| ServerRole {
        id: r.id,
        server_id: r.server_id,
        name: r.name,
        color: r.color,
        permissions: r.permissions,
        position: r.position,
        created_at: r.created_at,
    })
    .collect();
    let assignments = sqlx::query!(
        r#"
        SELECT user_id, role_id FROM server_member_roles WHERE server_id = $1
        "#,
        server_id.uuid(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .into_iter()
    .map(|r| (UserId::new(r.user_id), r.role_id))
    .collect();
    let members: HashSet<UserId> = sqlx::query_scalar!(
        r#"SELECT user_id FROM server_members WHERE server_id = $1"#,
        server_id.uuid(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .into_iter()
    .map(UserId::new)
    .collect();
    Ok(Server::load(
        ServerId::new(row.id),
        ServerName::new(row.name),
        UserId::new(row.owner_id),
        row.description.map(ServerDescription::new),
        row.accent_color,
        row.created_at,
        row.updated_at,
        roles,
        assignments,
        members,
    ))
}

#[async_trait]
impl ServerUnitOfWork for PostgresServerUnitOfWork {
    async fn begin(&self, server_id: &ServerId) -> Result<Box<dyn ServerTransaction>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        lock_server(&mut tx, server_id).await?;
        Ok(Box::new(PostgresServerTransaction {
            tx,
            server_id: *server_id,
        }))
    }
}

#[async_trait]
impl ServerTransaction for PostgresServerTransaction {
    async fn load_server(&mut self) -> Result<Server, RepositoryError> {
        load_server(&mut self.tx, &self.server_id).await
    }

    async fn insert_role(&mut self, role: &ServerRole) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO server_roles (id, server_id, name, color, permissions, position, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            role.id,
            role.server_id,
            role.name,
            role.color,
            role.permissions,
            role.position,
            role.created_at,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn update_role(
        &mut self,
        role_id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
        permissions: i64,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE server_roles
            SET name = COALESCE($3, name), color = COALESCE($4, color), permissions = $5
            WHERE id = $1 AND server_id = $2
            "#,
            role_id,
            self.server_id.uuid(),
            name,
            color,
            permissions,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_role(&mut self, role_id: Uuid) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"DELETE FROM server_roles WHERE id = $1 AND server_id = $2"#,
            role_id,
            self.server_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn assign_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO server_member_roles (server_id, user_id, role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
            self.server_id.uuid(),
            user_id.uuid(),
            role_id,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn revoke_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM server_member_roles
            WHERE server_id = $1 AND user_id = $2 AND role_id = $3
            "#,
            self.server_id.uuid(),
            user_id.uuid(),
            role_id,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn swap_role_positions(&mut self, role_a: Uuid, role_b: Uuid) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE server_roles AS r
            SET position = o.position
            FROM server_roles AS o
            WHERE r.server_id = $1 AND o.server_id = $1
              AND ((r.id = $2 AND o.id = $3) OR (r.id = $3 AND o.id = $2))
            "#,
            self.server_id.uuid(),
            role_a,
            role_b,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() == 2)
    }

    async fn remove_member(&mut self, user_id: &UserId) -> Result<bool, RepositoryError> {
        let server_id = self.server_id;
        let tx = &mut self.tx;
        let result = sqlx::query!(
            r#"
                DELETE FROM server_members
                WHERE server_id = $1 AND user_id = $2
            "#,
            server_id.uuid(),
            user_id.uuid(),
        )
        .execute(&mut **tx)
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
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
                DELETE FROM server_member_roles
                WHERE server_id = $1 AND user_id = $2
            "#,
            server_id.uuid(),
            user_id.uuid(),
        )
        .execute(&mut **tx)
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
        .execute(&mut **tx)
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
        .fetch_all(&mut **tx)
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
        .fetch_all(&mut **tx)
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
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let now = Utc::now();
        let mut viewer_events = Vec::new();
        for w in watched {
            let count = count_stream_viewers(tx, w.channel_id, w.streamer_id).await?;
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
        save_outbox_records(tx, &viewer_events).await?;
        sqlx::query!(
            r#"
                DELETE FROM channel_muted_participants
                WHERE user_id = $1
                  AND channel_id IN (SELECT id FROM channels WHERE server_id = $2)
            "#,
            user_id.uuid(),
            server_id.uuid(),
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn load_channel(&mut self, channel_id: &ChannelId) -> Result<Channel, RepositoryError> {
        load_channel_for_update(&mut self.tx, channel_id).await
    }

    async fn save_mute_state(
        &mut self,
        channel_id: &ChannelId,
        user_id: &UserId,
        mute: Option<&ParticipantMute>,
    ) -> Result<(), RepositoryError> {
        match mute {
            Some(mute) => {
                sqlx::query!(
                    r#"
                    INSERT INTO channel_muted_participants
                        (channel_id, user_id, muted_by, muted_at, self_muted)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (channel_id, user_id) DO UPDATE
                        SET muted_by = EXCLUDED.muted_by,
                            muted_at = EXCLUDED.muted_at,
                            self_muted = EXCLUDED.self_muted
                    "#,
                    channel_id.uuid(),
                    user_id.uuid(),
                    mute.muted_by.uuid(),
                    mute.muted_at,
                    mute.self_muted,
                )
                .execute(&mut *self.tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
            None => {
                sqlx::query!(
                    r#"DELETE FROM channel_muted_participants WHERE channel_id = $1 AND user_id = $2"#,
                    channel_id.uuid(),
                    user_id.uuid(),
                )
                .execute(&mut *self.tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
        }
        Ok(())
    }

    async fn insert_channel(&mut self, channel: &Channel) -> Result<(), RepositoryError> {
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
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        for mute in channel.muted_participants_list() {
            sqlx::query!(
                r#"
                INSERT INTO channel_muted_participants
                    (channel_id, user_id, muted_by, muted_at, self_muted)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                channel.id().uuid(),
                mute.user_id.uuid(),
                mute.muted_by.uuid(),
                mute.muted_at,
                mute.self_muted,
            )
            .execute(&mut *self.tx)
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
                .execute(&mut *self.tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
        }
        Ok(())
    }

    async fn delete_channel(&mut self, channel_id: &ChannelId) -> Result<Vec<String>, RepositoryError> {
        sqlx::query!(
            r#"SELECT id FROM channels WHERE id = $1 FOR UPDATE"#,
            channel_id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"SELECT id FROM messages WHERE channel_id = $1 FOR UPDATE"#,
            channel_id.uuid(),
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let storage_keys = sqlx::query_scalar!(
            r#"
            WITH removed AS (
                DELETE FROM message_attachments a
                USING messages m
                WHERE m.id = a.message_id AND m.channel_id = $1
                RETURNING a.storage_key
            )
            SELECT storage_key AS "storage_key!" FROM removed
            "#,
            channel_id.uuid(),
        )
        .fetch_all(&mut *self.tx)
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
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!("DELETE FROM channels WHERE id = $1", channel_id.uuid())
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(storage_keys)
    }

    async fn delete_message(&mut self, message_id: &MessageId) -> Result<(), RepositoryError> {
        let locked = sqlx::query_scalar!(
            r#"SELECT id FROM messages WHERE id = $1 FOR UPDATE"#,
            message_id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if locked.is_none() {
            return Err(RepositoryError::NotFound);
        }
        let removed_keys = sqlx::query_scalar!(
            r#"
            DELETE FROM message_attachments WHERE message_id = $1
            RETURNING storage_key
            "#,
            message_id.uuid(),
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            INSERT INTO storage_cleanup_queue (storage_key)
            SELECT * FROM UNNEST($1::text[])
            ON CONFLICT (storage_key) DO NOTHING
            "#,
            &removed_keys,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(r#"DELETE FROM messages WHERE id = $1"#, message_id.uuid())
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn update_server_details(
        &mut self,
        name: &str,
        description: Option<&str>,
    ) -> Result<bool, RepositoryError> {
        let updated = sqlx::query!(
            r#"
            UPDATE servers
            SET name = $2, description = $3, updated_at = now()
            WHERE id = $1
            "#,
            self.server_id.uuid(),
            name,
            description,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(updated.rows_affected() > 0)
    }

    async fn update_server_owner(&mut self, owner_id: &UserId) -> Result<bool, RepositoryError> {
        let updated = sqlx::query!(
            r#"
            UPDATE servers
            SET owner_id = $2, updated_at = now()
            WHERE id = $1
            "#,
            self.server_id.uuid(),
            owner_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(updated.rows_affected() > 0)
    }

    async fn find_longest_member_excluding(&mut self, exclude: &UserId) -> Result<Option<UserId>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT sm.user_id
            FROM server_members sm
            INNER JOIN messaging_user_profiles p ON p.user_id = sm.user_id
            WHERE sm.server_id = $1
              AND sm.user_id <> $2
              AND p.active
              AND NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = sm.user_id)
            ORDER BY sm.joined_at ASC, sm.user_id ASC
            LIMIT 1
            "#,
            self.server_id.uuid(),
            exclude.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| UserId::new(r.user_id)))
    }

    async fn create_invite_code(
        &mut self,
        created_by: &UserId,
        code: &str,
        max_uses: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<InviteCode, RepositoryError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO server_invite_codes (code, server_id, created_by, max_uses, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING code, server_id, created_by, max_uses, uses, expires_at, created_at
            "#,
            code,
            self.server_id.uuid(),
            created_by.uuid(),
            max_uses,
            expires_at,
        )
        .fetch_one(&mut *self.tx)
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

    async fn revoke_invite_code(
        &mut self,
        code: &str,
        requester: &UserId,
        any_creator: bool,
    ) -> Result<bool, RepositoryError> {
        let updated = sqlx::query!(
            r#"
            UPDATE server_invite_codes
            SET revoked_at = now()
            WHERE code = $1 AND server_id = $2 AND revoked_at IS NULL
              AND (created_by = $3 OR $4)
            "#,
            code,
            self.server_id.uuid(),
            requester.uuid(),
            any_creator,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(updated.rows_affected() > 0)
    }

    async fn insert_dm_message(&mut self, message: &DmMessage) -> Result<(), RepositoryError> {
        insert_dm_message(&mut self.tx, message).await
    }

    async fn insert_server(&mut self, server: &Server) -> Result<(), RepositoryError> {
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
        .execute(&mut *self.tx)
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
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn accept_invite(
        &mut self,
        invite_message_id: &DmMessageId,
        user_id: &UserId,
    ) -> Result<(), RepositoryError> {
        let deleted = sqlx::query!(
            r#"
                DELETE FROM dm_messages
                WHERE id = $1 AND recipient_id = $2 AND invite_server_id = $3
            "#,
            invite_message_id.uuid(),
            user_id.uuid(),
            self.server_id.uuid(),
        )
        .execute(&mut *self.tx)
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
            self.server_id.uuid(),
            user_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn delete_server_by_owner(&mut self, owner_id: &UserId) -> Result<Option<Vec<String>>, RepositoryError> {
        sqlx::query!(
            r#"SELECT id FROM servers WHERE id = $1 FOR UPDATE"#,
            self.server_id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"SELECT id FROM channels WHERE server_id = $1 FOR UPDATE"#,
            self.server_id.uuid(),
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            SELECT m.id
            FROM messages m
            INNER JOIN channels c ON c.id = m.channel_id
            WHERE c.server_id = $1
            FOR UPDATE OF m
            "#,
            self.server_id.uuid(),
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let storage_keys = sqlx::query_scalar!(
            r#"
            WITH removed AS (
                DELETE FROM message_attachments a
                USING messages m, channels c
                WHERE m.id = a.message_id AND c.id = m.channel_id AND c.server_id = $1
                RETURNING a.storage_key
            )
            SELECT storage_key AS "storage_key!" FROM removed
            "#,
            self.server_id.uuid(),
        )
        .fetch_all(&mut *self.tx)
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
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
                DELETE FROM servers WHERE id = $1 AND owner_id = $2
            "#,
            self.server_id.uuid(),
            owner_id.uuid()
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        Ok(Some(storage_keys))
    }

    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError> {
        let records = crate::outbox::mapping::to_records(events, request_id)?;
        save_outbox_records(&mut self.tx, &records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
