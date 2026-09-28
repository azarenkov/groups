use actix_web::{HttpResponse, Responder, get, post, web};
use contracts::internal::attachments::{ClaimRequestV1, ClaimedAttachmentV1, ReleaseRequestV1};
use domain::user::id::UserId;
use serde::Deserialize;
use uuid::Uuid;

use media_domain::attachment::repository::AttachmentRepository;

use crate::state::AppState;
use crate::http_errors::HttpError;

#[post("/attachments/claim")]
async fn claim_attachments(
    app_state: web::Data<AppState>,
    body: web::Json<ClaimRequestV1>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    if body.attachment_ids.is_empty() || body.attachment_ids.len() > 10 {
        return Err(HttpError::new(
            "Invalid attachments".to_string(),
            actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            "ATTACHMENTS_INVALID",
        ));
    }
    let owner = UserId::new(body.owner_id);
    match app_state
        .attachment_repository
        .claim_owned(&body.attachment_ids, &owner, body.claim_token)
        .await
    {
        Ok(attachments) => {
            let items: Vec<ClaimedAttachmentV1> = attachments
                .into_iter()
                .map(|a| ClaimedAttachmentV1 {
                    id: a.id,
                    file_name: a.file_name,
                    content_type: a.content_type,
                    size_bytes: a.size_bytes,
                    storage_key: a.storage_key,
                    created_at: a.created_at,
                    voice_duration_ms: a.voice.as_ref().map(|v| v.duration_ms),
                    voice_waveform: a.voice.map(|v| v.waveform),
                    video_duration_ms: a.video_note.as_ref().map(|v| v.duration_ms),
                    video_size: a.video_note.as_ref().map(|v| v.size),
                    video_poster: a.video_note.map(|v| v.poster),
                })
                .collect();
            Ok(HttpResponse::Ok().json(items))
        }
        Err(ports::repository::RepositoryError::InvalidData(_)) => {
            Err(HttpError::new(
                "Invalid attachments".to_string(),
                actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
                "ATTACHMENTS_INVALID",
            ))
        }
        Err(e) => Err(HttpError::from(e)),
    }
}

#[post("/attachments/release")]
async fn release_attachments(
    app_state: web::Data<AppState>,
    body: web::Json<ReleaseRequestV1>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    if !body.attachment_ids.is_empty() {
        app_state
            .attachment_repository
            .release_claims_by_token(&body.attachment_ids, body.claim_token)
            .await
            .map_err(HttpError::from)?;
    }
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnclaimedQuery {
    owner_id: Uuid,
}

#[get("/attachments/{attachment_id}/unclaimed")]
async fn unclaimed_attachment(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<UnclaimedQuery>,
) -> Result<impl Responder, HttpError> {
    let owner = UserId::new(query.owner_id);
    match app_state
        .attachment_repository
        .find_unclaimed_owned(path.into_inner(), &owner)
        .await
        .map_err(HttpError::from)?
    {
        Some(a) => Ok(HttpResponse::Ok().json(ClaimedAttachmentV1 {
            id: a.id,
            file_name: a.file_name,
            content_type: a.content_type,
            size_bytes: a.size_bytes,
            storage_key: a.storage_key,
            created_at: a.created_at,
            voice_duration_ms: a.voice.as_ref().map(|v| v.duration_ms),
            voice_waveform: a.voice.map(|v| v.waveform),
            video_duration_ms: a.video_note.as_ref().map(|v| v.duration_ms),
            video_size: a.video_note.as_ref().map(|v| v.size),
            video_poster: a.video_note.map(|v| v.poster),
        })),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

pub fn internal_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        web::scope("/internal")
            .service(claim_attachments)
            .service(release_attachments)
            .service(unclaimed_attachment),
    );
}
