use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostgresConfig {
    #[serde(rename = "postgres_host")]
    host: String,
    #[serde(rename = "postgres_port")]
    port: u16,
    #[serde(rename = "postgres_user", default)]
    user: String,
    #[serde(rename = "postgres_password", default)]
    password: String,
    #[serde(rename = "postgres_db", default)]
    db: String,
    #[serde(rename = "postgres_options", default)]
    options: Option<String>,
    #[serde(rename = "postgres_max_connections")]
    max_connections: u32,
    #[serde(rename = "postgres_min_connections")]
    min_connections: u32,
    #[serde(rename = "postgres_connect_timeout_seconds")]
    connect_timeout_seconds: u64,
    #[serde(rename = "postgres_idle_timeout_seconds")]
    idle_timeout_seconds: u64,
    #[serde(rename = "postgres_max_lifetime_seconds")]
    max_lifetime_seconds: u64,
    #[serde(rename = "postgres_run_migrations", default = "default_run_migrations")]
    run_migrations: bool,
}

fn default_run_migrations() -> bool {
    true
}

impl PostgresConfig {
    pub fn run_migrations(&self) -> bool {
        self.run_migrations
    }

    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<PostgresConfig>()
    }

    pub fn from_env_for(service: &str) -> Result<Self, envy::Error> {
        let mut config = Self::from_env()?;
        config.apply_service_overrides(service, |key| std::env::var(key).ok());
        config.ensure_complete(service)?;
        Ok(config)
    }

    fn ensure_complete(&self, service: &str) -> Result<(), envy::Error> {
        let prefix = service.to_ascii_uppercase().replace('-', "_");
        for (field, value) in [("DB", &self.db), ("USER", &self.user)] {
            if value.is_empty() {
                return Err(envy::Error::Custom(format!(
                    "{prefix}_POSTGRES_{field} or POSTGRES_{field} must be set"
                )));
            }
        }
        Ok(())
    }

    fn apply_service_overrides(&mut self, service: &str, lookup: impl Fn(&str) -> Option<String>) {
        let prefix = service.to_ascii_uppercase().replace('-', "_");
        let value = |field: &str| lookup(&format!("{prefix}_POSTGRES_{field}")).filter(|v| !v.is_empty());
        if let Some(db) = value("DB") {
            self.db = db;
        }
        if let Some(user) = value("USER") {
            self.user = user;
        }
        if let Some(password) = value("PASSWORD") {
            self.password = password;
        }
    }

    pub fn database(&self) -> &str {
        &self.db
    }

    pub fn url(&self) -> String {
        let user = urlencoding::encode(&self.user);
        let password = urlencoding::encode(&self.password);
        let base = format!(
            "postgres://{}:{}@{}:{}/{}",
            user, password, self.host, self.port, self.db,
        );

        match &self.options {
            Some(value) => format!("{}?{}", base, value),
            None => base,
        }
    }

    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }

    pub fn min_connections(&self) -> u32 {
        self.min_connections
    }

    pub fn connect_timeout_seconds(&self) -> u64 {
        self.connect_timeout_seconds
    }

    pub fn idle_timeout_seconds(&self) -> u64 {
        self.idle_timeout_seconds
    }

    pub fn max_lifetime_seconds(&self) -> u64 {
        self.max_lifetime_seconds
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::PostgresConfig;

    fn base() -> PostgresConfig {
        PostgresConfig {
            host: "postgres".to_string(),
            port: 5432,
            user: "postgres".to_string(),
            password: "postgres".to_string(),
            db: "rift".to_string(),
            options: None,
            max_connections: 10,
            min_connections: 2,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
            run_migrations: true,
        }
    }

    #[test]
    fn service_variables_override_the_shared_connection() {
        let env: HashMap<&str, &str> = HashMap::from([
            ("MESSAGING_POSTGRES_DB", "rift_messaging"),
            ("MESSAGING_POSTGRES_USER", "messaging"),
            ("MESSAGING_POSTGRES_PASSWORD", "secret"),
            ("VOICE_POSTGRES_DB", "rift_voice"),
        ]);
        let mut config = base();
        config.apply_service_overrides("messaging", |key| env.get(key).map(|v| v.to_string()));
        assert_eq!(config.url(), "postgres://messaging:secret@postgres:5432/rift_messaging");
    }

    #[test]
    fn a_service_without_any_database_name_is_rejected() {
        let mut config = base();
        config.db = String::new();
        config.apply_service_overrides("voice", |_| None);
        let error = config.ensure_complete("voice").unwrap_err().to_string();
        assert!(error.contains("VOICE_POSTGRES_DB"), "{error}");
    }

    #[test]
    fn missing_or_empty_service_variables_keep_the_shared_values() {
        let env: HashMap<&str, &str> = HashMap::from([("LIVEKIT_POOLER_POSTGRES_DB", "")]);
        let mut config = base();
        config.apply_service_overrides("livekit-pooler", |key| env.get(key).map(|v| v.to_string()));
        assert_eq!(config.url(), "postgres://postgres:postgres@postgres:5432/rift");
    }
}
