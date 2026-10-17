use api_common::internal_client::{InternalCallError, InternalClient, expect_success, read_json, send};
use async_trait::async_trait;
use contracts::internal::attachments::{ClaimRequestV1, ClaimedAttachmentV1, ReleaseRequestV1};
use contracts::internal::stickers::{StickerInfoV1, StickerPackInfoV1};
use domain::{attachment::AttachmentLink, sticker::StickerSnapshot, user::id::UserId};
use messaging::ports::attachment_claims::{AttachmentClaimError, AttachmentClaims};
use messaging::ports::sticker_packs::{StickerPackDirectory, StickerPackInfo, StickerPackLookupError};
use uuid::Uuid;

pub struct MediaClaimsClient {
    media: InternalClient,
}

impl MediaClaimsClient {
    pub fn new(base_url: &str) -> Self {
        Self { media: InternalClient::new(base_url) }
    }
}

fn to_link(d: ClaimedAttachmentV1) -> AttachmentLink {
    AttachmentLink {
        id: d.id,
        file_name: d.file_name,
        content_type: d.content_type,
        size_bytes: d.size_bytes,
        storage_key: d.storage_key,
        created_at: d.created_at,
        voice: domain::attachment::VoiceMeta::from_columns(d.voice_duration_ms, d.voice_waveform),
        video_note: domain::attachment::VideoNoteMeta::from_columns(d.video_duration_ms, d.video_size, d.video_poster),
    }
}

fn unavailable(error: InternalCallError) -> AttachmentClaimError {
    AttachmentClaimError::Unavailable(error.to_string())
}

#[async_trait]
impl AttachmentClaims for MediaClaimsClient {
    async fn claim(&self, ids: &[Uuid], owner_id: &UserId, claim_token: Uuid) -> Result<Vec<AttachmentLink>, AttachmentClaimError> {
        let request = self.media.post("/internal/attachments/claim").json(&ClaimRequestV1 {
            attachment_ids: ids.to_vec(),
            owner_id: owner_id.uuid(),
            claim_token,
        });
        let response = send(request).await.map_err(unavailable)?;
        match response.status().as_u16() {
            200 => {
                let items: Vec<ClaimedAttachmentV1> = read_json(response).await.map_err(unavailable)?;
                Ok(items.into_iter().map(to_link).collect())
            }
            422 => Err(AttachmentClaimError::Invalid),
            status => Err(unavailable(InternalCallError::Status { endpoint: "claim", status })),
        }
    }

    async fn lookup_unclaimed(&self, id: Uuid, owner_id: &UserId) -> Result<Option<AttachmentLink>, AttachmentClaimError> {
        let request = self
            .media
            .get(&format!("/internal/attachments/{}/unclaimed?ownerId={}", id, owner_id.uuid()));
        let response = send(request).await.map_err(unavailable)?;
        match response.status().as_u16() {
            200 => {
                let dto: ClaimedAttachmentV1 = read_json(response).await.map_err(unavailable)?;
                Ok(Some(to_link(dto)))
            }
            404 => Ok(None),
            status => Err(unavailable(InternalCallError::Status { endpoint: "unclaimed lookup", status })),
        }
    }

    async fn release(&self, ids: &[Uuid], claim_token: Uuid) -> Result<(), AttachmentClaimError> {
        let request = self
            .media
            .post("/internal/attachments/release")
            .json(&ReleaseRequestV1 { attachment_ids: ids.to_vec(), claim_token });
        expect_success(request, "release").await.map_err(unavailable)?;
        Ok(())
    }

    async fn lookup_sticker(&self, id: Uuid) -> Result<Option<StickerSnapshot>, AttachmentClaimError> {
        let response = send(self.media.get(&format!("/internal/stickers/{id}"))).await.map_err(unavailable)?;
        match response.status().as_u16() {
            200 => {
                let info: StickerInfoV1 = read_json(response).await.map_err(unavailable)?;
                if info.deleted {
                    return Ok(None);
                }
                Ok(Some(StickerSnapshot {
                    id: info.id,
                    pack_id: info.pack_id,
                    emoji: info.emoji,
                    animated: info.animated,
                    width: info.width,
                    height: info.height,
                }))
            }
            404 => Ok(None),
            status => Err(unavailable(InternalCallError::Status { endpoint: "sticker lookup", status })),
        }
    }
}

#[async_trait]
impl StickerPackDirectory for MediaClaimsClient {
    async fn find_pack(&self, id: Uuid) -> Result<Option<StickerPackInfo>, StickerPackLookupError> {
        let response = send(self.media.get(&format!("/internal/sticker-packs/{id}")))
            .await
            .map_err(|e| StickerPackLookupError(e.to_string()))?;
        match response.status().as_u16() {
            200 => {
                let info: StickerPackInfoV1 = read_json(response).await.map_err(|e| StickerPackLookupError(e.to_string()))?;
                Ok(Some(StickerPackInfo { id: info.id, owner_id: UserId::new(info.owner_id), name: info.name }))
            }
            404 => Ok(None),
            status => Err(StickerPackLookupError(
                InternalCallError::Status { endpoint: "sticker pack lookup", status }.to_string(),
            )),
        }
    }
}
