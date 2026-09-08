use std::{error::Error, time::Duration};

use sqlx::{Executor, Pool, Postgres, postgres::PgPoolOptions};

use crate::config::postgres::PostgresConfig;

pub async fn create_postgres_pool(
    config: PostgresConfig,
) -> Result<Pool<Postgres>, Box<dyn Error>> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections())
        .min_connections(config.min_connections())
        .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds()))
        .idle_timeout(Duration::from_secs(config.idle_timeout_seconds()))
        .max_lifetime(Duration::from_secs(config.max_lifetime_seconds()))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute(
                    "SET search_path TO public, identity, media, messaging, voice, realtime",
                )
                .await?;
                Ok(())
            })
        })
        .connect(&config.url())
        .await
        .map_err(Box::<dyn Error>::from)?;

    health_check(&pool).await?;

    if config.run_migrations() {
        sqlx::migrate!("../../../migrations")
            .run(&pool)
            .await
            .map_err(Box::<dyn Error>::from)?;
    }

    Ok(pool)
}

async fn health_check(pool: &Pool<Postgres>) -> Result<(), Box<dyn Error>> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(Box::<dyn Error>::from)?;

    Ok(())
}

const PING_TIMEOUT: Duration = Duration::from_secs(1);

pub async fn ping_pool(pool: &Pool<Postgres>) -> bool {
    matches!(
        tokio::time::timeout(PING_TIMEOUT, sqlx::query("SELECT 1").execute(pool)).await,
        Ok(Ok(_))
    )
}
