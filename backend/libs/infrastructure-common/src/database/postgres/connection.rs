use std::{error::Error, sync::Arc, time::Duration};

use sqlx::{Executor, Pool, Postgres, migrate::Migrator, postgres::PgPoolOptions};

use crate::config::postgres::PostgresConfig;

pub async fn create_postgres_pool(
    config: PostgresConfig,
    schema: &str,
    migrator: Option<&Migrator>,
) -> Result<Pool<Postgres>, Box<dyn Error>> {
    if !is_valid_schema(schema) {
        return Err(format!("invalid postgres schema name: {schema:?}").into());
    }
    let search_path: Arc<str> = Arc::from(format!("SET search_path TO {schema}"));
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections())
        .min_connections(config.min_connections())
        .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds()))
        .idle_timeout(Duration::from_secs(config.idle_timeout_seconds()))
        .max_lifetime(Duration::from_secs(config.max_lifetime_seconds()))
        .after_connect(move |conn, _meta| {
            let search_path = search_path.clone();
            Box::pin(async move {
                conn.execute(&*search_path).await?;
                Ok(())
            })
        })
        .connect(&config.url())
        .await
        .map_err(Box::<dyn Error>::from)?;

    health_check(&pool).await?;

    if let Some(migrator) = migrator
        && config.run_migrations()
    {
        pool.execute(format!("CREATE SCHEMA IF NOT EXISTS {schema}").as_str())
            .await
            .map_err(Box::<dyn Error>::from)?;
        migrator.run(&pool).await.map_err(Box::<dyn Error>::from)?;
    }

    Ok(pool)
}

pub fn is_valid_schema(schema: &str) -> bool {
    let mut chars = schema.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && schema.len() <= 63
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

#[cfg(test)]
mod tests {
    use super::is_valid_schema;

    #[test]
    fn accepts_service_schema_names() {
        for name in ["identity", "media", "messaging", "voice", "realtime", "_svc2"] {
            assert!(is_valid_schema(name), "{name}");
        }
    }

    #[test]
    fn rejects_names_that_would_need_quoting() {
        for name in ["", "Messaging", "2voice", "voice;drop", "voice schema", "voice-svc", "\"voice\""] {
            assert!(!is_valid_schema(name), "{name}");
        }
    }
}
