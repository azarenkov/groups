pub mod projection;
pub mod sequence_progress;

pub const SCHEMA: &str = "realtime";

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");
