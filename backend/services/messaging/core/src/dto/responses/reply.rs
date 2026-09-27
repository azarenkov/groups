use messaging_domain::message::{reply::ReplyPreview, reply::snippet, view::ReplyPreviewView};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ReplyPreviewDto {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub content: String,
    pub has_attachments: bool,
}

impl ReplyPreviewDto {
    pub fn from_preview(preview: &ReplyPreview, author_name: Option<String>) -> Self {
        Self {
            id: preview.message_id,
            author_id: preview.author_id.uuid(),
            author_name,
            content: preview.snippet.clone(),
            has_attachments: preview.has_attachments,
        }
    }
}

impl From<ReplyPreviewView> for ReplyPreviewDto {
    fn from(value: ReplyPreviewView) -> Self {
        Self {
            id: value.message_id,
            author_id: value.author_id.uuid(),
            author_name: Some(value.author_name),
            content: snippet(&value.content),
            has_attachments: value.has_attachments,
        }
    }
}
