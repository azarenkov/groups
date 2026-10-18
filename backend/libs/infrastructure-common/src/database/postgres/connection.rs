use std::{error::Error, sync::Arc, time::Duration};

use sqlx::{Executor, Pool, Postgres, migrate::Migrator, postgres::PgPoolOptions};

use crate::config::postgres::PostgresConfig;

pub async fn create_postgres_pool(
    config: PostgresConfig,
    schema: &str,
    migrator: Option<&Migrator>,
) -> Result<Pool<Postgres>, Box<dyn Error>> {
    let Some(statements) = SchemaStatements::for_schema(schema) else {
        return Err(format!("unknown postgres schema: {schema:?}").into());
    };
    let search_path: Arc<str> = Arc::from(statements.search_path);
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
        pool.execute(statements.create_schema)
            .await
            .map_err(Box::<dyn Error>::from)?;
        migrator.run(&pool).await.map_err(Box::<dyn Error>::from)?;
    }

    Ok(pool)
}

pub const KNOWN_SCHEMAS: &[&str] = &["identity", "media", "messaging", "voice", "realtime"];

pub fn is_known_schema(schema: &str) -> bool {
    KNOWN_SCHEMAS.contains(&schema)
}

struct SchemaStatements {
    create_schema: &'static str,
    search_path: &'static str,
}

impl SchemaStatements {
    fn for_schema(schema: &str) -> Option<Self> {
        let (create_schema, search_path) = match schema {
            "identity" => ("CREATE SCHEMA IF NOT EXISTS identity", "SET search_path TO identity"),
            "media" => ("CREATE SCHEMA IF NOT EXISTS media", "SET search_path TO media"),
            "messaging" => ("CREATE SCHEMA IF NOT EXISTS messaging", "SET search_path TO messaging"),
            "voice" => ("CREATE SCHEMA IF NOT EXISTS voice", "SET search_path TO voice"),
            "realtime" => ("CREATE SCHEMA IF NOT EXISTS realtime", "SET search_path TO realtime"),
            _ => return None,
        };
        Some(Self { create_schema, search_path })
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
    use super::{SchemaStatements, is_known_schema, is_valid_schema, KNOWN_SCHEMAS};

    #[test]
    fn only_known_schemas_get_literal_statements() {
        for name in KNOWN_SCHEMAS {
            assert!(is_known_schema(name), "{name}");
            let statements = SchemaStatements::for_schema(name).expect(name);
            assert!(statements.create_schema.ends_with(name));
            assert!(statements.search_path.ends_with(name));
        }
        assert!(!is_known_schema("_svc2"));
        assert!(SchemaStatements::for_schema("_svc2").is_none());
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
