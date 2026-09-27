use domain::attachment::AttachmentLink;

pub fn voice_message_is_valid(content: &str, attachments: &[AttachmentLink]) -> bool {
    if !attachments.iter().any(|a| a.voice.is_some()) {
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
