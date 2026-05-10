use uuid::Uuid;

#[derive(Debug)]
pub struct UnmuteChannelRequest {
    pub channel_id: Uuid,
    pub server_id: Uuid,
    pub target_user_id: Uuid,
}

impl UnmuteChannelRequest {
    pub fn new(channel_id: Uuid, server_id: Uuid, target_user_id: Uuid) -> Self {
        Self {
            channel_id,
            server_id,
            target_user_id,
        }
    }
}
