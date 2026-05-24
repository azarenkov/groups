use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DispatcherConfig {
    #[serde(default = "default_health_port")]
    pub health_port: u16,
    #[serde(default = "default_user_stream")]
    pub dispatcher_user_stream: String,
    #[serde(default = "default_channel_stream")]
    pub dispatcher_channel_stream: String,
    #[serde(default = "default_message_stream")]
    pub dispatcher_message_stream: String,
    #[serde(default = "default_dm_message_stream")]
    pub dispatcher_dm_message_stream: String,
    #[serde(default = "default_server_stream")]
    pub dispatcher_server_stream: String,
    #[serde(default = "default_consumer_prefix")]
    pub dispatcher_consumer_prefix: String,
    #[serde(default = "default_max_deliver")]
    pub dispatcher_max_deliver: u32,
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

fn default_server_stream() -> String {
    "RIFT_SERVER_EVENTS".to_string()
}

fn default_consumer_prefix() -> String {
    "event-dispatcher".to_string()
}

fn default_max_deliver() -> u32 {
    10
}
