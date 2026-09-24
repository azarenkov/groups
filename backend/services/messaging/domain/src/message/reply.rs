use domain::user::id::UserId;
use uuid::Uuid;

pub const REPLY_SNIPPET_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyPreview {
    pub message_id: Uuid,
    pub author_id: UserId,
    pub snippet: String,
    pub has_attachments: bool,
}

impl ReplyPreview {
    pub fn new(message_id: Uuid, author_id: UserId, content: &str, has_attachments: bool) -> Self {
        Self {
            message_id,
            author_id,
            snippet: snippet(content),
            has_attachments,
        }
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
