use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, delete, get, post, web};
use domain::{server::id::ServerId, user::id::UserId};
use futures::StreamExt;
use image::{ImageDecoder, ImageFormat, imageops::FilterType};
use api_common::middleware::auth::{AuthToken, RateLimitedAuthToken};
use uuid::Uuid;

use crate::state::AppState;
use crate::http_errors::HttpError;

pub const MAX_AVATAR_BYTES: usize = 8 * 1024 * 1024;
const MIN_AVATAR_DIMENSION: u32 = 128;
const AVATAR_SIZE: u32 = 512;
const MAX_DECODE_DIMENSION: u32 = 8192;
const MAX_DECODE_ALLOC: u64 = 64 * 1024 * 1024;

pub(crate) struct ImageCodes {
    pub too_large: &'static str,
    pub empty: &'static str,
    pub invalid: &'static str,
    pub type_not_allowed: &'static str,
    pub too_small: &'static str,
}

const AVATAR_CODES: ImageCodes = ImageCodes {
    too_large: "AVATAR_TOO_LARGE",
    empty: "AVATAR_EMPTY",
    invalid: "AVATAR_INVALID_IMAGE",
    type_not_allowed: "AVATAR_TYPE_NOT_ALLOWED",
    too_small: "AVATAR_TOO_SMALL",
};

pub(crate) async fn read_image_field(
    payload: &mut Multipart,
    max_bytes: usize,
    codes: &ImageCodes,
) -> Result<Vec<u8>, HttpError> {
    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| HttpError::bad_request(e.to_string()))?;
        if field.name() != Some("file") {
            continue;
        }
        let mut bytes: Vec<u8> = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| HttpError::bad_request(e.to_string()))?;
            if bytes.len() + chunk.len() > max_bytes {
                return Err(HttpError::new(
                    format!("Image is too large (max {} MB).", max_bytes / (1024 * 1024)),
                    actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
                    codes.too_large,
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(HttpError::new(
                "Image is empty.".to_string(),
                actix_web::http::StatusCode::BAD_REQUEST,
                codes.empty,
            ));
        }
        return Ok(bytes);
    }
    Err(HttpError::bad_request("missing file field".to_string()))
}

pub(crate) fn invalid_image(code: &'static str, message: &str) -> HttpError {
    HttpError::new(
        message.to_string(),
        actix_web::http::StatusCode::BAD_REQUEST,
        code,
    )
}

pub(crate) fn decode_image(
    bytes: &[u8],
    codes: &ImageCodes,
) -> Result<image::DynamicImage, HttpError> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| invalid_image(codes.invalid, "Unreadable image."))?;
    match reader.format() {
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP) => {}
        _ => {
            return Err(HttpError::new(
                "Only PNG, JPEG or WebP images are allowed.".to_string(),
                actix_web::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                codes.type_not_allowed,
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
        .map_err(|_| invalid_image(codes.invalid, "Invalid image."))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder)
        .map_err(|_| invalid_image(codes.invalid, "Invalid image."))?;
    img.apply_orientation(orientation);
    Ok(img)
}

pub(crate) fn encode_png(
    img: &image::DynamicImage,
    codes: &ImageCodes,
) -> Result<Vec<u8>, HttpError> {
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|_| invalid_image(codes.invalid, "Failed to process image."))?;
    Ok(out)
}

fn normalize_image(bytes: Vec<u8>) -> Result<Vec<u8>, HttpError> {
    let img = decode_image(&bytes, &AVATAR_CODES)?;
    let (w, h) = (img.width(), img.height());
    if w < MIN_AVATAR_DIMENSION || h < MIN_AVATAR_DIMENSION {
        return Err(invalid_image(
            AVATAR_CODES.too_small,
            "Image must be at least 128×128 pixels.",
        ));
    }
    let side = w.min(h);
    let img = img
        .crop_imm((w - side) / 2, (h - side) / 2, side, side)
        .resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);
    encode_png(&img, &AVATAR_CODES)
}

async fn read_and_normalize(
    app_state: &AppState,
    payload: &mut Multipart,
) -> Result<Vec<u8>, HttpError> {
    let _permit = app_state
        .upload_semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| HttpError::bad_request("upload capacity unavailable".to_string()))?;
    let bytes = read_image_field(payload, MAX_AVATAR_BYTES, &AVATAR_CODES).await?;
    web::block(move || normalize_image(bytes))
        .await
        .map_err(|_| HttpError::bad_request("image processing failed".to_string()))?
}

pub(crate) fn image_response(stream: media::ports::file_storage::ByteStream) -> HttpResponse {
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    app_state
        .delete_user_avatar_usecase
        .execute(user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub(crate) async fn check_image_read_limit(
    app_state: &AppState,
    requester: uuid::Uuid,
) -> Result<(), HttpError> {
    if !app_state.image_read_limiter.check_key(&requester).await {
        return Err(HttpError::new(
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
    check_image_read_limit(&app_state, auth_token.id).await?;
    let server_id = ServerId::new(path.into_inner());
    let stream = app_state.get_server_logo_usecase.execute(server_id).await?;
    Ok(image_response(stream))
}
