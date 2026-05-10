use uuid::Uuid;

#[derive(Debug)]
pub struct LeaveChannelRequest {
    pub channel_id: Uuid,
    pub server_id: Uuid,
}

impl LeaveChannelRequest {
    pub fn new(channel_id: Uuid, server_id: Uuid) -> Self {
        Self {
            channel_id,
            server_id,
        }
    }
}
