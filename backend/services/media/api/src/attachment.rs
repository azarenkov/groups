use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, delete, post, web};
use domain::user::id::UserId;
use futures::StreamExt;
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

const MAX_FILE_NAME_BYTES: usize = 255;
const UPLOAD_CHANNEL_CHUNKS: usize = 16;

use crate::state::AppState;
use api_common::dto::responses::ApiResponse;
use api_common::dto::responses::attachment::AttachmentResponse;
use crate::http_errors::HttpError;

#[post("")]
async fn upload_attachment(
    req: actix_web::HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let limits = app_state.upload_attachment_usecase.limits_for(&user_id).await?;
    let declared_bytes = req
        .headers()
        .get(actix_web::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(limits.max_request_bytes());
    if declared_bytes > limits.max_request_bytes() {
        return Err(HttpError::payload_too_large());
    }

    let _permit = app_state
        .upload_semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| HttpError::bad_request("upload capacity unavailable".to_string()))?;

    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| HttpError::bad_request(e.to_string()))?;
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

        let (sender, receiver) = tokio::sync::mpsc::channel::<
            Result<Vec<u8>, media::ports::file_storage::FileStorageError>,
        >(UPLOAD_CHANNEL_CHUNKS);
        let stream: media::ports::file_storage::ByteStream =
            Box::pin(futures::stream::unfold(receiver, |mut receiver| async move {
                receiver.recv().await.map(|chunk| (chunk, receiver))
            }));

        let pump = async {
            while let Some(chunk) = field.next().await {
                let message = chunk.map(|bytes| bytes.to_vec()).map_err(|e| {
                    media::ports::file_storage::FileStorageError::Internal(e.to_string())
                });
                let failed = message.is_err();
                if sender.send(message).await.is_err() || failed {
                    break;
                }
            }
            drop(sender);
        };
        let upload = app_state.upload_attachment_usecase.execute(
            user_id,
            file_name,
            content_type,
            declared_bytes,
            limits,
            stream,
        );
        let ((), attachment) = futures::join!(pump, upload);
        let attachment = attachment?;

        return Ok(HttpResponse::Created()
            .json(ApiResponse::success(AttachmentResponse::from(attachment))));
    }

    Err(HttpError::bad_request("missing file field".to_string()))
}

#[delete("/{attachment_id}")]
async fn delete_attachment(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
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
