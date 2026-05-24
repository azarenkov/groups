use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RealtimeConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_health_port")]
    pub health_port: u16,
    pub jwt_secret: String,
    #[serde(default = "default_user_stream")]
    pub realtime_user_stream: String,
    #[serde(default = "default_channel_stream")]
    pub realtime_channel_stream: String,
    #[serde(default = "default_message_stream")]
    pub realtime_message_stream: String,
    #[serde(default = "default_dm_message_stream")]
    pub realtime_dm_message_stream: String,
    #[serde(default = "default_server_stream")]
    pub realtime_server_stream: String,
    #[serde(default = "default_consumer_prefix")]
    pub realtime_consumer_prefix: String,
    #[serde(default = "default_outbound_buffer")]
    pub realtime_outbound_buffer: usize,
}

impl RealtimeConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<RealtimeConfig>()
    }
}

fn default_bind_address() -> String {
    "0.0.0.0:8083".to_string()
}

fn default_health_port() -> u16 {
    8084
}

fn default_user_stream() -> String {
    "RIFT_USER_EVENTS".to_string()
}

fn default_channel_stream() -> String {
    "RIFT_CHANNEL_EVENTS".to_string()
}

fn default_message_stream() -> String {
    "RIFT_MESSAGE_EVENTS".to_string()
}

fn default_dm_message_stream() -> String {
    "RIFT_DM_MESSAGE_EVENTS".to_string()
}

fn default_server_stream() -> String {
    "RIFT_SERVER_EVENTS".to_string()
}

fn default_consumer_prefix() -> String {
    "realtime-service".to_string()
}

fn default_outbound_buffer() -> usize {
    64
}
