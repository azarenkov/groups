use uuid::Uuid;

#[derive(Debug)]
pub struct JoinChannelRequest {
    pub channel_id: Uuid,
    pub server_id: Uuid,
    pub password: Option<String>,
}

impl JoinChannelRequest {
    pub fn new(channel_id: Uuid, server_id: Uuid, password: Option<String>) -> Self {
        Self {
            channel_id,
            server_id,
            password,
        }
    }
}
