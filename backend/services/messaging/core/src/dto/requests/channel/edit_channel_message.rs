use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct EditChannelMessageRequest {
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub new_content: String,
    pub add_attachment_ids: Vec<Uuid>,
    pub remove_attachment_ids: Vec<Uuid>,
    pub mention_ids: Vec<Uuid>,
    pub ctx: RequestContext,
}

impl EditChannelMessageRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        server_id: Uuid,
        channel_id: Uuid,
        message_id: Uuid,
        new_content: String,
        add_attachment_ids: Vec<Uuid>,
        remove_attachment_ids: Vec<Uuid>,
        mention_ids: Vec<Uuid>,
        ctx: RequestContext,
    ) -> Self {
        Self {
            server_id,
            channel_id,
            message_id,
            new_content,
            add_attachment_ids,
            remove_attachment_ids,
            mention_ids,
            ctx,
        }
    }
}
