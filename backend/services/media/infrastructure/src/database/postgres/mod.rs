pub mod repositories;
pub use infrastructure_common::database::postgres::{connection, error};

pub const SCHEMA: &str = "media";

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");
