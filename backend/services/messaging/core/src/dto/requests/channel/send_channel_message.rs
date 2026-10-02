use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendChannelMessageRequest {
    pub community_id: Uuid,
    pub channel_id: Uuid,
    pub content: String,
    pub attachment_ids: Vec<Uuid>,
    pub reply_to_message_id: Option<Uuid>,
    pub forward: Option<crate::dto::requests::forward::ForwardedContent>,
    pub sticker_id: Option<Uuid>,
    pub ctx: RequestContext,
}

impl SendChannelMessageRequest {
    pub fn new(
        community_id: Uuid,
        channel_id: Uuid,
        content: String,
        attachment_ids: Vec<Uuid>,
        reply_to_message_id: Option<Uuid>,
        ctx: RequestContext,
    ) -> Self {
        Self {
            community_id,
            channel_id,
            content,
            attachment_ids,
            reply_to_message_id,
            forward: None,
            sticker_id: None,
            ctx,
        }
    }

    pub fn with_forward(mut self, forward: crate::dto::requests::forward::ForwardedContent) -> Self {
        self.reply_to_message_id = None;
        self.attachment_ids = Vec::new();
        self.sticker_id = None;
        self.forward = Some(forward);
        self
    }

    pub fn with_sticker(mut self, sticker_id: Option<Uuid>) -> Self {
        self.sticker_id = sticker_id;
        self
    }
}
