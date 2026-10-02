use domain::{attachment::AttachmentLink, sticker::StickerSnapshot};
use messaging_domain::message::forward::ForwardInfo;

#[derive(Debug, Clone)]
pub struct ForwardedContent {
    pub info: ForwardInfo,
    pub content: String,
    pub attachments: Vec<AttachmentLink>,
    pub sticker: Option<StickerSnapshot>,
}
