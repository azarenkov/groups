use domain::user::id::UserId;
use uuid::Uuid;

pub const REPLY_SNIPPET_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyPreview {
    pub message_id: Uuid,
    pub author_id: UserId,
    pub snippet: String,
    pub has_attachments: bool,
    pub voice_duration_ms: Option<i32>,
    pub video_note_duration_ms: Option<i32>,
    pub video_note_poster: Option<Vec<u8>>,
    pub hidden_for: Vec<UserId>,
}

impl ReplyPreview {
    pub fn new(message_id: Uuid, author_id: UserId, content: &str, has_attachments: bool) -> Self {
        Self {
            message_id,
            author_id,
            snippet: snippet(content),
            has_attachments,
            voice_duration_ms: None,
            video_note_duration_ms: None,
            video_note_poster: None,
            hidden_for: Vec::new(),
        }
    }

    pub fn with_voice(mut self, voice_duration_ms: Option<i32>) -> Self {
        self.voice_duration_ms = voice_duration_ms;
        self
    }

    pub fn with_video_note(mut self, note: Option<&domain::attachment::VideoNoteMeta>) -> Self {
        self.video_note_duration_ms = note.map(|n| n.duration_ms);
        self.video_note_poster = note.map(|n| n.poster.clone());
        self
    }

    pub fn hidden_for(mut self, users: Vec<UserId>) -> Self {
        self.hidden_for = users;
        self
    }
}

pub fn snippet(content: &str) -> String {
    let flat = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= REPLY_SNIPPET_CHARS {
        return flat;
    }
    let mut cut: String = flat.chars().take(REPLY_SNIPPET_CHARS).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_flattens_line_breaks_and_spaces() {
        assert_eq!(snippet("  first line\n\nsecond   line \n"), "first line second line");
    }

    #[test]
    fn snippet_cuts_long_text_with_an_ellipsis() {
        let long = "a".repeat(REPLY_SNIPPET_CHARS + 50);
        let cut = snippet(&long);
        assert_eq!(cut.chars().count(), REPLY_SNIPPET_CHARS + 1);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn snippet_keeps_short_text_untouched() {
        assert_eq!(snippet("привет"), "привет");
    }
}
