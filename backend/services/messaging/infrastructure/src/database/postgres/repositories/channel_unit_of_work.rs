use async_trait::async_trait;
use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{channel::Channel, channel::VoiceChange, channel::events::ChannelEvent, channel::id::ChannelId, channel::unit_of_work::ChannelTransaction, channel::unit_of_work::ChannelUnitOfWork, events::MessagingEvent};
use ports::{outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres, Transaction};

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::{
        channel::{
            apply_media_changes, apply_voice_changes, cleanup_session_stream_viewer,
            clear_stream_for_streamer, count_stream_viewers, load_channel_for_update,
            recompute_session_watched_streams, stream_viewers_record,
        },
        outbox_record::save_outbox_records,
    },
};

pub struct PostgresChannelUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresChannelUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresChannelTransaction {
    tx: Transaction<'static, Postgres>,
}

#[async_trait]
impl ChannelUnitOfWork for PostgresChannelUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn ChannelTransaction>, RepositoryError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(Box::new(PostgresChannelTransaction { tx }))
    }
}

#[async_trait]
impl ChannelTransaction for PostgresChannelTransaction {
    async fn claim_inbox(&mut self, source: &str, external_id: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO webhook_inbox (source, external_id)
            VALUES (($1::text)::webhook_source, $2)
            ON CONFLICT DO NOTHING
            "#,
            source,
            external_id,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn load_channel_for_update(&mut self, channel_id: &ChannelId) -> Result<Channel, RepositoryError> {
        load_channel_for_update(&mut self.tx, channel_id).await
    }

    async fn save_voice_state(
        &mut self,
        channel: &mut Channel,
        events: &[ChannelEvent],
        request_id: Option<&str>,
    ) -> Result<(), RepositoryError> {
        let request_id = request_id.map(str::to_string);
        let departed: Vec<uuid::Uuid> = events
            .iter()
            .filter_map(|e| e.departed_user().map(|u| u.uuid()))
            .collect();
        let screen_stopped: Vec<uuid::Uuid> = events
            .iter()
            .filter_map(|e| e.screen_stopped_user().map(|u| u.uuid()))
            .collect();
        let screen_started: Vec<uuid::Uuid> = events
            .iter()
            .filter_map(|e| e.screen_started_user().map(|u| u.uuid()))
            .collect();
        let voice_changes = channel.pull_voice_changes();
        let media_changes = channel.pull_media_changes();
        let community_id = channel.community_id().uuid();
        let channel_uuid = channel.id().uuid();
        let tx = &mut self.tx;


        let removed_sessions: Vec<(uuid::Uuid, String)> = voice_changes
            .iter()
            .filter_map(|vc| match vc {
                VoiceChange::Removed { user_id, session_id } => {
                    Some((user_id.uuid(), session_id.as_str().to_string()))
                }
                VoiceChange::Added { .. } => None,
            })
            .collect();
        let added_sessions: Vec<(uuid::Uuid, String)> = voice_changes
            .iter()
            .filter_map(|vc| match vc {
                VoiceChange::Added { user_id, session_id, .. } => {
                    Some((user_id.uuid(), session_id.as_str().to_string()))
                }
                VoiceChange::Removed { .. } => None,
            })
            .collect();

        apply_voice_changes(tx, channel.id(), &voice_changes).await?;
        apply_media_changes(tx, channel.id(), &media_changes).await?;

        let mut records: Vec<OutboxRecord> = Vec::new();
        for (viewer, session) in added_sessions {
            records.extend(
                recompute_session_watched_streams(tx, channel_uuid, community_id, viewer, &session, Utc::now(), request_id.clone())
                    .await?,
            );
        }
        for (viewer, session) in removed_sessions {
            records.extend(
                cleanup_session_stream_viewer(tx, channel_uuid, community_id, viewer, &session, Utc::now(), request_id.clone())
                    .await?,
            );
        }
        for streamer in departed.into_iter().chain(screen_stopped) {
            if let Some(record) =
                clear_stream_for_streamer(tx, channel_uuid, community_id, streamer, Utc::now(), request_id.clone()).await?
            {
                records.push(record);
            }
        }
        for streamer in screen_started {
            let count = count_stream_viewers(tx, channel_uuid, streamer).await?;
            records.push(stream_viewers_record(channel_uuid, community_id, streamer, count, Utc::now(), request_id.clone()));
        }
        save_outbox_records(tx, &records).await
    }

    async fn join_text(&mut self, channel_id: &ChannelId, user_id: &UserId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO channel_text_participants (channel_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            channel_id.uuid(),
            user_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError> {
        let records = crate::outbox::mapping::to_records(events, request_id)?;
        save_outbox_records(&mut self.tx, &records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
