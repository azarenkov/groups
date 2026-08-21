use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, delete, get, post, web};
use domain::{server::id::ServerId, user::id::UserId};
use futures::StreamExt;
use image::{ImageDecoder, ImageFormat, imageops::FilterType};
use shared::middleware::auth::{AuthToken, RateLimitedAuthToken};
use uuid::Uuid;

use crate::state::AppState;
use shared::api_error::ApiError;

pub const MAX_AVATAR_BYTES: usize = 8 * 1024 * 1024;
const MIN_AVATAR_DIMENSION: u32 = 128;
const AVATAR_SIZE: u32 = 512;
const MAX_DECODE_DIMENSION: u32 = 8192;
const MAX_DECODE_ALLOC: u64 = 64 * 1024 * 1024;

async fn read_image_field(payload: &mut Multipart) -> Result<Vec<u8>, ApiError> {
    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| ApiError::bad_request(e.to_string()))?;
        if field.name() != Some("file") {
            continue;
        }
        let mut bytes: Vec<u8> = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| ApiError::bad_request(e.to_string()))?;
            if bytes.len() + chunk.len() > MAX_AVATAR_BYTES {
                return Err(ApiError::new(
                    "Image is too large (max 8 MB).".to_string(),
                    actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
                    "AVATAR_TOO_LARGE",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(ApiError::new(
                "Image is empty.".to_string(),
                actix_web::http::StatusCode::BAD_REQUEST,
                "AVATAR_EMPTY",
            ));
        }
        return Ok(bytes);
    }
    Err(ApiError::bad_request("missing file field".to_string()))
}

fn normalize_image(bytes: Vec<u8>) -> Result<Vec<u8>, ApiError> {
    let invalid = |code: &'static str, message: &str| {
        ApiError::new(
            message.to_string(),
            actix_web::http::StatusCode::BAD_REQUEST,
            code,
        )
    };
    let mut reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|_| invalid("AVATAR_INVALID_IMAGE", "Unreadable image."))?;
    match reader.format() {
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP) => {}
        _ => {
            return Err(ApiError::new(
                "Only PNG, JPEG or WebP images are allowed.".to_string(),
                actix_web::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "AVATAR_TYPE_NOT_ALLOWED",
            ));
        }
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| invalid("AVATAR_INVALID_IMAGE", "Invalid image."))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder)
        .map_err(|_| invalid("AVATAR_INVALID_IMAGE", "Invalid image."))?;
    img.apply_orientation(orientation);
    let (w, h) = (img.width(), img.height());
    if w < MIN_AVATAR_DIMENSION || h < MIN_AVATAR_DIMENSION {
        return Err(invalid(
            "AVATAR_TOO_SMALL",
            "Image must be at least 128×128 pixels.",
        ));
    }
    let side = w.min(h);
    let img = img
        .crop_imm((w - side) / 2, (h - side) / 2, side, side)
        .resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|_| invalid("AVATAR_INVALID_IMAGE", "Failed to process image."))?;
    Ok(out)
}

async fn read_and_normalize(
    app_state: &AppState,
    payload: &mut Multipart,
) -> Result<Vec<u8>, ApiError> {
    let _permit = app_state
        .upload_semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| ApiError::bad_request("upload capacity unavailable".to_string()))?;
    let bytes = read_image_field(payload).await?;
    web::block(move || normalize_image(bytes))
        .await
        .map_err(|_| ApiError::bad_request("image processing failed".to_string()))?
}

fn image_response(stream: media::ports::file_storage::ByteStream) -> HttpResponse {
    let body = stream.map(|chunk| chunk.map(web::Bytes::from));
    HttpResponse::Ok()
        .content_type("image/png")
        .insert_header(("X-Content-Type-Options", "nosniff"))
        .insert_header(("Cache-Control", "private, no-store"))
        .streaming(body)
}

#[post("/me/avatar")]
async fn upload_my_avatar(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let normalized = read_and_normalize(&app_state, &mut payload).await?;
    app_state
        .update_user_avatar_usecase
        .execute(user_id, normalized)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/me/avatar")]
async fn delete_my_avatar(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    app_state
        .delete_user_avatar_usecase
        .execute(user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn check_image_read_limit(
    app_state: &AppState,
    requester: uuid::Uuid,
) -> Result<(), ApiError> {
    if !app_state.image_read_limiter.check_key(&requester).await {
        return Err(ApiError::new(
            "Too many image requests. Slow down.".to_string(),
            actix_web::http::StatusCode::TOO_MANY_REQUESTS,
            "IMAGE_RATE_LIMITED",
        ));
    }
    Ok(())
}

#[get("/{user_id}/avatar")]
async fn get_user_avatar(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    check_image_read_limit(&app_state, auth_token.id).await?;
    let user_id = UserId::new(path.into_inner());
    let stream = app_state.get_user_avatar_usecase.execute(user_id).await?;
    Ok(image_response(stream))
}

#[post("/{server_id}/logo")]
async fn upload_server_logo(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    mut payload: Multipart,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);
    let normalized = read_and_normalize(&app_state, &mut payload).await?;
    app_state
        .update_server_logo_usecase
        .execute(server_id, user_id, Some(normalized))
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{server_id}/logo")]
async fn delete_server_logo(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);
    app_state
        .update_server_logo_usecase
        .execute(server_id, user_id, None)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{server_id}/logo")]
async fn get_server_logo(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    check_image_read_limit(&app_state, auth_token.id).await?;
    let server_id = ServerId::new(path.into_inner());
    let stream = app_state.get_server_logo_usecase.execute(server_id).await?;
    Ok(image_response(stream))
}
