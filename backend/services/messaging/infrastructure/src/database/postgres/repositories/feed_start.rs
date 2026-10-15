use async_trait::async_trait;
use chrono::{DateTime, Utc};
use messaging::ports::feed_start::{Feed, FeedStarts};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresFeedStarts {
    pool: Pool<Postgres>,
}

impl PostgresFeedStarts {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FeedStarts for PostgresFeedStarts {
    async fn started_at(&self, feed: Feed) -> Result<DateTime<Utc>, RepositoryError> {
        sqlx::query_scalar!("SELECT started_at FROM feed_starts WHERE feed = $1", feed.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)
    }
}
