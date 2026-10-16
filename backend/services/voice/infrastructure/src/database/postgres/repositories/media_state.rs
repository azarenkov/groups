use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use voice_domain::media::{MediaRevision, MediaState, repository::MediaStateRepository};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresMediaStateRepository {
    pool: Pool<Postgres>,
}

impl PostgresMediaStateRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MediaStateRepository for PostgresMediaStateRepository {
    async fn find(&self, user_id: &UserId) -> Result<Option<MediaState>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT mic_muted, deafened FROM voice_media_states WHERE user_id = $1"#,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| MediaState::new(r.mic_muted, r.deafened)))
    }

    async fn save(
        &self,
        user_id: &UserId,
        state: MediaState,
        revision: Option<MediaRevision>,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO voice_media_states (user_id, mic_muted, deafened, source, seq, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id) DO UPDATE
            SET mic_muted = EXCLUDED.mic_muted, deafened = EXCLUDED.deafened,
                source = EXCLUDED.source, seq = EXCLUDED.seq, updated_at = EXCLUDED.updated_at
            WHERE voice_media_states.source IS DISTINCT FROM EXCLUDED.source
               OR voice_media_states.seq IS NULL
               OR EXCLUDED.seq > voice_media_states.seq
            "#,
            user_id.uuid(),
            state.mic_muted(),
            state.deafened(),
            revision.map(|r| r.source),
            revision.map(|r| r.seq),
            now,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
