use actix_web::{HttpResponse, Responder, post, web};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use domain::attachment::repository::AttachmentRepository;

use crate::media::state::AppState;
use crate::shared::error::ApiError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaimBody {
    attachment_ids: Vec<Uuid>,
    owner_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseBody {
    attachment_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimedAttachment {
    id: Uuid,
    file_name: String,
    content_type: String,
    size_bytes: i64,
    storage_key: String,
    created_at: DateTime<Utc>,
}

#[post("/attachments/claim")]
async fn claim_attachments(
    app_state: web::Data<AppState>,
    body: web::Json<ClaimBody>,
) -> Result<impl Responder, ApiError> {
    let body = body.into_inner();
    if body.attachment_ids.is_empty() || body.attachment_ids.len() > 10 {
        return Err(ApiError::new(
            "Invalid attachments".to_string(),
            actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            "ATTACHMENTS_INVALID",
        ));
    }
    let owner = UserId::new(body.owner_id);
    match app_state
        .attachment_repository
        .claim_owned(&body.attachment_ids, &owner)
        .await
    {
        Ok(attachments) => {
            let items: Vec<ClaimedAttachment> = attachments
                .into_iter()
                .map(|a| ClaimedAttachment {
                    id: a.id,
                    file_name: a.file_name,
                    content_type: a.content_type,
                    size_bytes: a.size_bytes,
                    storage_key: a.storage_key,
                    created_at: a.created_at,
                })
                .collect();
            Ok(HttpResponse::Ok().json(items))
        }
        Err(domain::shared::repository::error::RepositoryError::InvalidData(_)) => {
            Err(ApiError::new(
                "Invalid attachments".to_string(),
                actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
                "ATTACHMENTS_INVALID",
            ))
        }
        Err(e) => Err(ApiError::from(e)),
    }
}

#[post("/attachments/release")]
async fn release_attachments(
    app_state: web::Data<AppState>,
    body: web::Json<ReleaseBody>,
) -> Result<impl Responder, ApiError> {
    let body = body.into_inner();
    if !body.attachment_ids.is_empty() {
        app_state
            .attachment_repository
            .release_claims(&body.attachment_ids)
            .await
            .map_err(ApiError::from)?;
    }
    Ok(HttpResponse::NoContent().finish())
}

pub fn internal_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        web::scope("/internal")
            .service(claim_attachments)
            .service(release_attachments),
    );
}
