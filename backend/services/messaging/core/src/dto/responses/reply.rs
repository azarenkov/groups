use messaging_domain::message::{reply::ReplyPreview, reply::snippet, view::ReplyPreviewView};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ReplyPreviewDto {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub content: String,
    pub has_attachments: bool,
    pub voice_duration_ms: Option<i32>,
    pub video_note_duration_ms: Option<i32>,
    pub video_note_poster: Option<String>,
    pub sticker: Option<domain::sticker::StickerSnapshot>,
}

fn poster_base64(poster: &Option<Vec<u8>>) -> Option<String> {
    use base64::Engine;
    poster
        .as_ref()
        .map(|p| base64::engine::general_purpose::STANDARD.encode(p))
}

impl ReplyPreviewDto {
    pub fn from_preview(preview: &ReplyPreview, author_name: Option<String>) -> Self {
        Self {
            id: preview.message_id,
            author_id: preview.author_id.uuid(),
            author_name,
            content: preview.snippet.clone(),
            has_attachments: preview.has_attachments,
            voice_duration_ms: preview.voice_duration_ms,
            video_note_duration_ms: preview.video_note_duration_ms,
            video_note_poster: poster_base64(&preview.video_note_poster),
            sticker: preview.sticker.clone(),
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
            voice_duration_ms: value.voice_duration_ms,
            video_note_duration_ms: value.video_note_duration_ms,
            video_note_poster: poster_base64(&value.video_note_poster),
            sticker: value.sticker,
        }
    }
}
