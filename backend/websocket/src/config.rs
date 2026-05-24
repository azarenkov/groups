use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WebsocketConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_health_port")]
    pub health_port: u16,
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

fn default_health_port() -> u16 {
    8084
}

fn default_outbound_buffer() -> usize {
    64
}
