use std::sync::Arc;

use chrono::Utc;
use domain::{attachment::Attachment, attachment::VoiceMeta, user::id::UserId};
use ports::repository::RepositoryError;
use media_domain::attachment::repository::AttachmentRepository;
use uuid::Uuid;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use futures::StreamExt;
use media_domain::banner::BannerRepository;

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};

const MEGABYTE: u64 = 1024 * 1024;
pub const MAX_UNLINKED_COUNT: i64 = 50;
const MIN_UNLINKED_BYTES: u64 = 200 * MEGABYTE;
const MULTIPART_OVERHEAD_BYTES: u64 = 64 * 1024;

const MINUTE_MS: i32 = 60 * 1000;
pub const VOICE_TYPES: &[&str] = &["audio/webm", "audio/ogg", "audio/mp4"];
pub const VOICE_WAVEFORM_MIN: usize = 32;
pub const VOICE_WAVEFORM_MAX: usize = 128;
const VOICE_MAX_BYTES_PER_SECOND: u64 = 48 * 1024;
const VOICE_HEAD_BYTES: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadLimits {
    pub max_file_bytes: u64,
    pub max_unlinked_bytes: u64,
    pub max_voice_ms: i32,
}

impl UploadLimits {
    pub fn for_tier(tier: &str) -> Self {
        let (max_file_bytes, per_message) = match tier {
            "premium" => (500 * MEGABYTE, 20),
            "pro" => (100 * MEGABYTE, 10),
            _ => (15 * MEGABYTE, 5),
        };
        let max_voice_ms = match tier {
            "premium" | "pro" => 30 * MINUTE_MS,
            _ => 5 * MINUTE_MS,
        };
        Self {
            max_file_bytes,
            max_unlinked_bytes: MIN_UNLINKED_BYTES.max(max_file_bytes * per_message),
            max_voice_ms,
        }
    }

    pub fn max_request_bytes(&self) -> u64 {
        self.max_file_bytes + MULTIPART_OVERHEAD_BYTES
    }
}

const ALLOWED_PREFIXES: &[&str] = &["image/", "video/", "audio/", "text/"];
const ALLOWED_TYPES: &[&str] = &[
    "application/pdf",
    "application/zip",
    "application/x-zip-compressed",
    "application/gzip",
    "application/x-tar",
    "application/x-7z-compressed",
    "application/json",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-powerpoint",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/octet-stream",
];

pub fn base_content_type(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

pub fn validate_voice(
    content_type: &str,
    voice: &VoiceMeta,
    limits: &UploadLimits,
) -> Result<(), UploadAttachmentError> {
    if !VOICE_TYPES.contains(&base_content_type(content_type).as_str()) {
        return Err(UploadAttachmentError::InvalidVoice);
    }
    if voice.duration_ms <= 0 || voice.duration_ms > limits.max_voice_ms {
        return Err(UploadAttachmentError::VoiceTooLong);
    }
    if !(VOICE_WAVEFORM_MIN..=VOICE_WAVEFORM_MAX).contains(&voice.waveform.len()) {
        return Err(UploadAttachmentError::InvalidVoice);
    }
    Ok(())
}

pub fn voice_signature_matches(content_type: &str, head: &[u8]) -> bool {
    match base_content_type(content_type).as_str() {
        "audio/webm" => head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        "audio/ogg" => head.starts_with(b"OggS"),
        "audio/mp4" => head.len() >= 8 && &head[4..8] == b"ftyp",
        _ => false,
    }
}

pub fn voice_size_plausible(bytes: u64, duration_ms: i32) -> bool {
    let seconds = (duration_ms.max(0) as u64).div_ceil(1000) + 2;
    bytes <= seconds * VOICE_MAX_BYTES_PER_SECOND
}

fn content_type_allowed(content_type: &str) -> bool {
    ALLOWED_PREFIXES.iter().any(|p| content_type.starts_with(p))
        || ALLOWED_TYPES.contains(&content_type)
}

#[derive(Debug, thiserror::Error)]
pub enum UploadAttachmentError {
    #[error("File is too large")]
    TooLarge,
    #[error("File is empty")]
    Empty,
    #[error("File type is not allowed")]
    TypeNotAllowed,
    #[error("Too many pending uploads")]
    QuotaExceeded,
    #[error("Invalid voice message")]
    InvalidVoice,
    #[error("Voice message is too long")]
    VoiceTooLong,
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct UploadAttachment<R: AttachmentRepository, S: FileStorage> {
    repository: Arc<R>,
    storage: Arc<S>,
    plans: Arc<dyn BannerRepository>,
}

impl<R: AttachmentRepository, S: FileStorage> UploadAttachment<R, S> {
    pub fn new(repository: Arc<R>, storage: Arc<S>, plans: Arc<dyn BannerRepository>) -> Self {
        Self {
            repository,
            storage,
            plans,
        }
    }

    pub async fn limits_for(&self, owner_id: &UserId) -> Result<UploadLimits, UploadAttachmentError> {
        let tier = self.plans.tier_of(owner_id, Utc::now()).await?;
        Ok(UploadLimits::for_tier(&tier))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        owner_id: UserId,
        file_name: String,
        content_type: String,
        declared_bytes: u64,
        limits: UploadLimits,
        voice: Option<VoiceMeta>,
        stream: ByteStream,
    ) -> Result<Attachment, UploadAttachmentError> {
        if declared_bytes == 0 {
            return Err(UploadAttachmentError::Empty);
        }
        if declared_bytes > limits.max_request_bytes() {
            return Err(UploadAttachmentError::TooLarge);
        }
        let content_type = match &voice {
            Some(meta) => {
                validate_voice(&content_type, meta, &limits)?;
                base_content_type(&content_type)
            }
            None => content_type,
        };
        if !content_type_allowed(&content_type) {
            return Err(UploadAttachmentError::TypeNotAllowed);
        }

        let id = Uuid::new_v4();
        let storage_key = format!("attachments/{id}");
        let mut attachment = Attachment {
            id,
            owner_id: owner_id.uuid(),
            file_name,
            content_type,
            size_bytes: declared_bytes.min(limits.max_file_bytes) as i64,
            storage_key,
            created_at: Utc::now(),
            voice: voice.clone(),
        };

        let reserved = self
            .repository
            .insert_reserving_quota(
                &attachment,
                MAX_UNLINKED_COUNT,
                limits.max_unlinked_bytes as i64,
            )
            .await?;
        if !reserved {
            return Err(UploadAttachmentError::QuotaExceeded);
        }

        let too_large = Arc::new(AtomicBool::new(false));
        let bad_voice = Arc::new(AtomicBool::new(false));
        let voice_checked = Arc::new(AtomicBool::new(voice.is_none()));
        let seen = Arc::new(AtomicU64::new(0));
        let limited: ByteStream = {
            let too_large = too_large.clone();
            let bad_voice = bad_voice.clone();
            let voice_checked = voice_checked.clone();
            let seen = seen.clone();
            let max = limits.max_file_bytes;
            let head = Mutex::new(Vec::with_capacity(VOICE_HEAD_BYTES));
            let content_type = attachment.content_type.clone();
            Box::pin(stream.map(move |chunk| {
                let chunk = chunk?;
                let total = seen.fetch_add(chunk.len() as u64, Ordering::Relaxed) + chunk.len() as u64;
                if total > max {
                    too_large.store(true, Ordering::Relaxed);
                    return Err(FileStorageError::Internal("file exceeds the plan limit".to_string()));
                }
                if !voice_checked.load(Ordering::Relaxed) {
                    let mut head = head.lock().unwrap_or_else(|e| e.into_inner());
                    let take = (VOICE_HEAD_BYTES - head.len()).min(chunk.len());
                    head.extend_from_slice(&chunk[..take]);
                    if head.len() >= VOICE_HEAD_BYTES {
                        voice_checked.store(true, Ordering::Relaxed);
                        if !voice_signature_matches(&content_type, &head) {
                            bad_voice.store(true, Ordering::Relaxed);
                            return Err(FileStorageError::Internal("voice signature mismatch".to_string()));
                        }
                    }
                }
                Ok(chunk)
            }))
        };

        let written = self
            .storage
            .put_stream(&attachment.storage_key, &attachment.content_type, limited)
            .await;
        let failure = match written {
            Ok(0) => Some(UploadAttachmentError::Empty),
            Ok(_) if !voice_checked.load(Ordering::Relaxed) => Some(UploadAttachmentError::InvalidVoice),
            Ok(bytes)
                if voice
                    .as_ref()
                    .is_some_and(|meta| !voice_size_plausible(bytes, meta.duration_ms)) =>
            {
                Some(UploadAttachmentError::InvalidVoice)
            }
            Ok(bytes) => match self.repository.set_size(attachment.id, bytes as i64).await {
                Ok(()) => {
                    attachment.size_bytes = bytes as i64;
                    None
                }
                Err(e) => Some(e.into()),
            },
            Err(_) if too_large.load(Ordering::Relaxed) => Some(UploadAttachmentError::TooLarge),
            Err(_) if bad_voice.load(Ordering::Relaxed) => Some(UploadAttachmentError::InvalidVoice),
            Err(e) => Some(e.into()),
        };
        let Some(failure) = failure else {
            return Ok(attachment);
        };

        match self
            .repository
            .enqueue_cleanup(std::slice::from_ref(&attachment.storage_key))
            .await
        {
            Ok(()) => {
                let _ = self.repository.delete_orphan_row(attachment.id).await;
            }
            Err(qe) => {
                tracing::warn!(error = %qe, key = %attachment.storage_key, "Failed to enqueue cleanup after upload error; orphan sweep will reclaim");
            }
        }
        Err(failure)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        UploadAttachmentError, UploadLimits, validate_voice, voice_signature_matches,
        voice_size_plausible,
    };
    use domain::attachment::VoiceMeta;

    fn voice(duration_ms: i32, bars: usize) -> VoiceMeta {
        VoiceMeta { duration_ms, waveform: vec![10; bars] }
    }

    #[test]
    fn a_voice_note_fits_the_plan_length() {
        let free = UploadLimits::for_tier("free");
        let pro = UploadLimits::for_tier("pro");
        assert!(validate_voice("audio/webm;codecs=opus", &voice(5 * 60 * 1000, 64), &free).is_ok());
        assert!(matches!(
            validate_voice("audio/webm", &voice(5 * 60 * 1000 + 1, 64), &free),
            Err(UploadAttachmentError::VoiceTooLong)
        ));
        assert!(validate_voice("audio/mp4", &voice(30 * 60 * 1000, 64), &pro).is_ok());
        assert!(matches!(
            validate_voice("audio/ogg", &voice(0, 64), &pro),
            Err(UploadAttachmentError::VoiceTooLong)
        ));
    }

    #[test]
    fn a_voice_note_needs_an_audio_container_and_a_sane_waveform() {
        let free = UploadLimits::for_tier("free");
        assert!(matches!(
            validate_voice("audio/mpeg", &voice(1000, 64), &free),
            Err(UploadAttachmentError::InvalidVoice)
        ));
        assert!(matches!(
            validate_voice("audio/webm", &voice(1000, 8), &free),
            Err(UploadAttachmentError::InvalidVoice)
        ));
        assert!(matches!(
            validate_voice("audio/webm", &voice(1000, 500), &free),
            Err(UploadAttachmentError::InvalidVoice)
        ));
    }

    #[test]
    fn the_file_signature_must_match_the_declared_container() {
        let webm = [0x1A, 0x45, 0xDF, 0xA3, 0, 0, 0, 0, 0, 0, 0, 0];
        let ogg = *b"OggS\0\x02\0\0\0\0\0\0";
        let mp4 = *b"\0\0\0\x20ftypM4A ";
        assert!(voice_signature_matches("audio/webm;codecs=opus", &webm));
        assert!(voice_signature_matches("audio/ogg", &ogg));
        assert!(voice_signature_matches("audio/mp4", &mp4));
        assert!(!voice_signature_matches("audio/mp4", &webm));
        assert!(!voice_signature_matches("audio/webm", b"<html><body>"));
    }

    #[test]
    fn a_voice_file_cannot_be_far_bigger_than_its_length() {
        assert!(voice_size_plausible(200 * 1024, 10_000));
        assert!(!voice_size_plausible(50 * 1024 * 1024, 10_000));
    }

    const MEGABYTE: u64 = 1024 * 1024;

    #[test]
    fn a_plan_sets_the_largest_file() {
        assert_eq!(UploadLimits::for_tier("free").max_file_bytes, 15 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("pro").max_file_bytes, 100 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("premium").max_file_bytes, 500 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("gold").max_file_bytes, 15 * MEGABYTE);
    }

    #[test]
    fn pending_uploads_leave_room_for_a_full_message() {
        assert_eq!(UploadLimits::for_tier("free").max_unlinked_bytes, 200 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("pro").max_unlinked_bytes, 1000 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("premium").max_unlinked_bytes, 10_000 * MEGABYTE);
    }

    #[test]
    fn a_request_may_carry_a_little_more_than_the_file() {
        let limits = UploadLimits::for_tier("free");
        assert!(limits.max_request_bytes() > limits.max_file_bytes);
        assert!(limits.max_request_bytes() < limits.max_file_bytes + MEGABYTE);
    }
}
