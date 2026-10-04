use actix_web::{HttpResponse, Responder, get, web};
use domain::user::id::UserId;
use futures::StreamExt;
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::state::AppState;
use crate::http_errors::HttpError;

#[get("/{attachment_id}")]
async fn download_attachment(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let attachment_id = path.into_inner();
    let user_id = UserId::new(auth_token.id);

    let (attachment, stream) = app_state
        .get_attachment_usecase
        .execute(attachment_id, user_id)
        .await?;

    let inline = attachment.content_type.starts_with("image/");
    let ascii_name: String = attachment
        .file_name
        .chars()
        .map(|c| {
            if c.is_ascii() && c != '"' && c != '\\' && !c.is_ascii_control() {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded_name: String = attachment
        .file_name
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    let kind = if inline { "inline" } else { "attachment" };
    let disposition =
        format!("{kind}; filename=\"{ascii_name}\"; filename*=UTF-8''{encoded_name}");

    let body = stream.map(|chunk| chunk.map(web::Bytes::from));
    Ok(HttpResponse::Ok()
        .content_type(attachment.content_type)
        .insert_header(("Content-Disposition", disposition))
        .insert_header(("X-Content-Type-Options", "nosniff"))
        .insert_header(("Cache-Control", "private, no-store"))
        .streaming(body))
}

#[get("/{attachment_id}/thumbnail")]
async fn attachment_thumbnail(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let (content_type, stream) = app_state
        .get_attachment_thumbnail_usecase
        .execute(path.into_inner(), UserId::new(auth_token.id))
        .await?;
    let body = stream.map(|chunk| chunk.map(web::Bytes::from));
    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .insert_header(("Content-Disposition", "inline"))
        .insert_header(("X-Content-Type-Options", "nosniff"))
        .insert_header(("Cache-Control", "private, no-store"))
        .streaming(body))
}

pub fn attachment_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/attachments")
            .service(attachment_thumbnail)
            .service(download_attachment),
    );
}
