use std::{error::Error, time::Duration};

use sqlx::{Executor, Pool, Postgres, migrate::Migrator, postgres::PgPoolOptions};

use crate::config::postgres::PostgresConfig;

pub async fn create_postgres_pool(
    config: PostgresConfig,
    schema: &str,
    migrator: Option<&Migrator>,
) -> Result<Pool<Postgres>, Box<dyn Error>> {
    let Some(known) = KnownSchema::parse(schema) else {
        return Err(format!("unknown postgres schema: {schema:?}").into());
    };
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections())
        .min_connections(config.min_connections())
        .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds()))
        .idle_timeout(Duration::from_secs(config.idle_timeout_seconds()))
        .max_lifetime(Duration::from_secs(config.max_lifetime_seconds()))
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                known.set_search_path(conn).await?;
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
        known.create(&pool).await.map_err(Box::<dyn Error>::from)?;
        migrator.run(&pool).await.map_err(Box::<dyn Error>::from)?;
    }

    Ok(pool)
}

pub const KNOWN_SCHEMAS: &[&str] = &["identity", "media", "messaging", "voice", "realtime"];

pub fn is_known_schema(schema: &str) -> bool {
    KnownSchema::parse(schema).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownSchema {
    Identity,
    Media,
    Messaging,
    Voice,
    Realtime,
}

impl KnownSchema {
    pub fn parse(schema: &str) -> Option<Self> {
        Some(match schema {
            "identity" => Self::Identity,
            "media" => Self::Media,
            "messaging" => Self::Messaging,
            "voice" => Self::Voice,
            "realtime" => Self::Realtime,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Media => "media",
            Self::Messaging => "messaging",
            Self::Voice => "voice",
            Self::Realtime => "realtime",
        }
    }

    async fn set_search_path(self, conn: &mut sqlx::PgConnection) -> Result<(), sqlx::Error> {
        match self {
            Self::Identity => conn.execute("SET search_path TO identity").await?,
            Self::Media => conn.execute("SET search_path TO media").await?,
            Self::Messaging => conn.execute("SET search_path TO messaging").await?,
            Self::Voice => conn.execute("SET search_path TO voice").await?,
            Self::Realtime => conn.execute("SET search_path TO realtime").await?,
        };
        Ok(())
    }

    async fn create(self, pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
        match self {
            Self::Identity => pool.execute("CREATE SCHEMA IF NOT EXISTS identity").await?,
            Self::Media => pool.execute("CREATE SCHEMA IF NOT EXISTS media").await?,
            Self::Messaging => pool.execute("CREATE SCHEMA IF NOT EXISTS messaging").await?,
            Self::Voice => pool.execute("CREATE SCHEMA IF NOT EXISTS voice").await?,
            Self::Realtime => pool.execute("CREATE SCHEMA IF NOT EXISTS realtime").await?,
        };
        Ok(())
    }
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
    use super::{KnownSchema, is_known_schema, is_valid_schema, KNOWN_SCHEMAS};

    #[test]
    fn only_known_schemas_are_accepted() {
        for name in KNOWN_SCHEMAS {
            assert!(is_known_schema(name), "{name}");
            assert_eq!(KnownSchema::parse(name).map(KnownSchema::name), Some(*name));
        }
        assert!(!is_known_schema("_svc2"));
        assert!(KnownSchema::parse("_svc2").is_none());
    }

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
