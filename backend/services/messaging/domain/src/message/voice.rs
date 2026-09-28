use domain::attachment::{AttachmentLink, VideoNoteMeta};

pub fn is_rich_media(link: &AttachmentLink) -> bool {
    link.voice.is_some() || link.video_note.is_some()
}

pub fn video_note_of(attachments: &[AttachmentLink]) -> Option<&VideoNoteMeta> {
    attachments.iter().find_map(|a| a.video_note.as_ref())
}

pub fn voice_message_is_valid(content: &str, attachments: &[AttachmentLink]) -> bool {
    if !attachments.iter().any(is_rich_media) {
        return true;
    }
    attachments.len() == 1 && content.trim().is_empty()
}

pub fn voice_duration_of(attachments: &[AttachmentLink]) -> Option<i32> {
    attachments
        .iter()
        .find_map(|a| a.voice.as_ref().map(|v| v.duration_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use domain::attachment::VoiceMeta;
    use uuid::Uuid;

    fn link(voice: bool) -> AttachmentLink {
        AttachmentLink {
            id: Uuid::new_v4(),
            file_name: "a".into(),
            content_type: "audio/webm".into(),
            size_bytes: 10,
            storage_key: "k".into(),
            created_at: Utc::now(),
            voice: voice.then(|| VoiceMeta { duration_ms: 1200, waveform: vec![1; 64] }),
            video_note: None,
        }
    }

    #[test]
    fn a_voice_note_travels_alone() {
        assert!(voice_message_is_valid("", &[link(true)]));
        assert!(voice_message_is_valid("  ", &[link(true)]));
        assert!(!voice_message_is_valid("caption", &[link(true)]));
        assert!(!voice_message_is_valid("", &[link(true), link(false)]));
    }

    #[test]
    fn a_video_note_travels_alone_too() {
        let mut note = link(false);
        note.video_note = Some(domain::attachment::VideoNoteMeta { duration_ms: 3000, size: 384, poster: vec![0xFF, 0xD8, 0xFF] });
        assert!(voice_message_is_valid("", std::slice::from_ref(&note)));
        assert!(!voice_message_is_valid("hi", std::slice::from_ref(&note)));
        assert!(!voice_message_is_valid("", &[note.clone(), link(false)]));
        assert_eq!(video_note_of(&[link(false), note]).map(|n| n.duration_ms), Some(3000));
    }

    #[test]
    fn ordinary_attachments_are_untouched() {
        assert!(voice_message_is_valid("text", &[link(false), link(false)]));
        assert!(voice_message_is_valid("text", &[]));
    }

    #[test]
    fn the_duration_comes_from_the_voice_attachment() {
        assert_eq!(voice_duration_of(&[link(false), link(true)]), Some(1200));
        assert_eq!(voice_duration_of(&[link(false)]), None);
    }
}
