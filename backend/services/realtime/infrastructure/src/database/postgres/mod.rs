pub mod projection;

pub const SCHEMA: &str = "realtime";

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");
