use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WebsocketConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_health_port")]
    pub health_port: u16,
    pub jwt_secret: String,
    #[serde(default = "default_user_stream")]
    pub websocket_user_stream: String,
    #[serde(default = "default_channel_stream")]
    pub websocket_channel_stream: String,
    #[serde(default = "default_message_stream")]
    pub websocket_message_stream: String,
    #[serde(default = "default_dm_message_stream")]
    pub websocket_dm_message_stream: String,
    #[serde(default = "default_server_stream")]
    pub websocket_server_stream: String,
    #[serde(default = "default_consumer_prefix")]
    pub websocket_consumer_prefix: String,
    #[serde(default = "default_outbound_buffer")]
    pub websocket_outbound_buffer: usize,
    #[serde(default = "default_max_deliver")]
    pub websocket_max_deliver: u32,
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
    "websocket".to_string()
}

fn default_outbound_buffer() -> usize {
    64
}

fn default_max_deliver() -> u32 {
    10
}
