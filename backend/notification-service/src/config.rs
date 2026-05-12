use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NotificationConfig {
    #[serde(default = "default_user_stream")]
    pub notification_user_stream: String,
    #[serde(default = "default_verification_consumer")]
    pub notification_verification_consumer: String,
}

impl NotificationConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<NotificationConfig>()
    }
}

fn default_user_stream() -> String {
    "RIFT_USER_EVENTS".to_string()
}

fn default_verification_consumer() -> String {
    "notification-service-verification".to_string()
}
