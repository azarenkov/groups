use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, delete, post, web};
use application::usecases::attachment::upload_attachment::MAX_ATTACHMENT_BYTES;
use domain::user::id::UserId;
use futures::StreamExt;
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

const MAX_FILE_NAME_BYTES: usize = 255;

use crate::media::state::AppState;
use crate::shared::{
    dto::responses::{ApiResponse, attachment::AttachmentResponse},
    error::ApiError,
    };

#[post("")]
async fn upload_attachment(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);

    let _permit = app_state
        .upload_semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| ApiError::bad_request("upload capacity unavailable".to_string()))?;

    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| ApiError::bad_request(e.to_string()))?;
        if field.name() != Some("file") {
            continue;
        }
        let raw_name = field
            .content_disposition()
            .and_then(|cd| cd.get_filename())
            .unwrap_or("file");
        let mut file_name = String::with_capacity(raw_name.len().min(MAX_FILE_NAME_BYTES));
        for c in raw_name.chars() {
            if file_name.len() + c.len_utf8() > MAX_FILE_NAME_BYTES {
                break;
            }
            file_name.push(c);
        }
        if file_name.trim().is_empty() {
            file_name = "file".to_string();
        }
        let content_type = field
            .content_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let mut bytes: Vec<u8> = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| ApiError::bad_request(e.to_string()))?;
            if bytes.len() + chunk.len() > MAX_ATTACHMENT_BYTES {
                return Err(ApiError::payload_too_large());
            }
            bytes.extend_from_slice(&chunk);
        }

        let attachment = app_state
            .upload_attachment_usecase
            .execute(user_id, file_name, content_type, bytes)
            .await?;

        return Ok(HttpResponse::Created()
            .json(ApiResponse::success(AttachmentResponse::from(attachment))));
    }

    Err(ApiError::bad_request("missing file field".to_string()))
}

#[delete("/{attachment_id}")]
async fn delete_attachment(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let attachment_id = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    app_state
        .delete_attachment_usecase
        .execute(attachment_id, user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn attachment_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/attachments")
            .service(upload_attachment)
            .service(delete_attachment),
    );
}
