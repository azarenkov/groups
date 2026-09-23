use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DispatcherConfig {
    #[serde(default = "default_health_port")]
    pub dispatcher_health_port: u16,
    #[serde(default = "default_user_stream")]
    pub dispatcher_user_stream: String,
    #[serde(default = "default_channel_stream")]
    pub dispatcher_channel_stream: String,
    #[serde(default = "default_message_stream")]
    pub dispatcher_message_stream: String,
    #[serde(default = "default_dm_message_stream")]
    pub dispatcher_dm_message_stream: String,
    #[serde(default = "default_community_stream")]
    pub dispatcher_community_stream: String,
    #[serde(default = "default_consumer_prefix")]
    pub dispatcher_consumer_prefix: String,
    #[serde(default = "default_max_deliver")]
    pub dispatcher_max_deliver: u32,
    #[serde(default = "default_liveness_stale_secs")]
    pub dispatcher_liveness_stale_secs: u64,
}

impl DispatcherConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<DispatcherConfig>()
    }
}

fn default_health_port() -> u16 {
    8085
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

fn default_community_stream() -> String {
    "RIFT_SERVER_EVENTS".to_string()
}

fn default_consumer_prefix() -> String {
    "event-dispatcher".to_string()
}

fn default_max_deliver() -> u32 {
    10
}

fn default_liveness_stale_secs() -> u64 {
    30
}

#[derive(Debug, Deserialize)]
pub struct WebsocketConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_websocket_health_port")]
    pub websocket_health_port: u16,
    pub jwt_secret: String,
    #[serde(default = "default_outbound_buffer")]
    pub websocket_outbound_buffer: usize,
}

impl WebsocketConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<WebsocketConfig>()
    }
}

fn default_bind_address() -> String {
    "0.0.0.0:8083".to_string()
}

fn default_websocket_health_port() -> u16 {
    8084
}

fn default_outbound_buffer() -> usize {
    64
}
