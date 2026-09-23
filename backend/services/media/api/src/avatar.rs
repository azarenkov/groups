use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, delete, get, post, web};
use domain::{community::id::CommunityId, user::id::UserId};
use futures::StreamExt;
use image::{ImageDecoder, ImageFormat, imageops::FilterType};
use api_common::middleware::auth::{AuthToken, RateLimitedAuthToken};
use media::usecases::user::update_avatar::NormalizedImage;
use tokio::sync::OwnedSemaphorePermit;
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
    pub bad_crop: &'static str,
}

const AVATAR_CODES: ImageCodes = ImageCodes {
    too_large: "AVATAR_TOO_LARGE",
    empty: "AVATAR_EMPTY",
    invalid: "AVATAR_INVALID_IMAGE",
    type_not_allowed: "AVATAR_TYPE_NOT_ALLOWED",
    too_small: "AVATAR_TOO_SMALL",
    bad_crop: "AVATAR_CROP_INVALID",
};

const ANIMATED_AVATAR_SIZE: u32 = 256;
const MAX_ANIMATION_FRAMES: usize = 240;
const CROP_ASPECT_TOLERANCE: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub(crate) struct ImageUpload {
    pub bytes: Vec<u8>,
    pub crop: Option<CropRect>,
}

pub(crate) fn is_gif(bytes: &[u8]) -> bool {
    bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")
}

pub(crate) fn resolve_crop(
    width: u32,
    height: u32,
    crop: Option<CropRect>,
    aspect_w: u32,
    aspect_h: u32,
    codes: &ImageCodes,
) -> Result<CropRect, HttpError> {
    let Some(crop) = crop else {
        let (crop_w, crop_h) =
            if u64::from(width) * u64::from(aspect_h) > u64::from(height) * u64::from(aspect_w) {
                (height * aspect_w / aspect_h, height)
            } else {
                (width, width * aspect_h / aspect_w)
            };
        return Ok(CropRect {
            x: (width - crop_w) / 2,
            y: (height - crop_h) / 2,
            width: crop_w,
            height: crop_h,
        });
    };
    let fits = crop.width > 0
        && crop.height > 0
        && u64::from(crop.x) + u64::from(crop.width) <= u64::from(width)
        && u64::from(crop.y) + u64::from(crop.height) <= u64::from(height);
    let wanted = f64::from(aspect_w) / f64::from(aspect_h);
    let actual = f64::from(crop.width) / f64::from(crop.height.max(1));
    if !fits || ((actual - wanted).abs() / wanted) > CROP_ASPECT_TOLERANCE {
        return Err(invalid_image(codes.bad_crop, "The crop area does not fit the image."));
    }
    Ok(crop)
}

pub(crate) async fn read_image_upload(
    payload: &mut Multipart,
    max_bytes: usize,
    codes: &ImageCodes,
) -> Result<ImageUpload, HttpError> {
    let mut bytes: Option<Vec<u8>> = None;
    let mut crop: [Option<u32>; 4] = [None; 4];
    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| HttpError::bad_request(e.to_string()))?;
        let slot = match field.name() {
            Some("file") => None,
            Some("cropX") => Some(0),
            Some("cropY") => Some(1),
            Some("cropWidth") => Some(2),
            Some("cropHeight") => Some(3),
            _ => continue,
        };
        let limit = if slot.is_some() { 16 } else { max_bytes };
        let mut buffer: Vec<u8> = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| HttpError::bad_request(e.to_string()))?;
            if buffer.len() + chunk.len() > limit {
                if slot.is_some() {
                    return Err(invalid_image(codes.bad_crop, "The crop area is not a number."));
                }
                return Err(HttpError::new(
                    format!("Image is too large (max {} MB).", max_bytes / (1024 * 1024)),
                    actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
                    codes.too_large,
                ));
            }
            buffer.extend_from_slice(&chunk);
        }
        match slot {
            Some(index) => {
                let value = std::str::from_utf8(&buffer)
                    .ok()
                    .and_then(|text| text.trim().parse::<u32>().ok())
                    .ok_or_else(|| {
                        invalid_image(codes.bad_crop, "The crop area is not a number.")
                    })?;
                crop[index] = Some(value);
            }
            None => bytes = Some(buffer),
        }
    }
    let bytes = bytes.ok_or_else(|| HttpError::bad_request("missing file field".to_string()))?;
    if bytes.is_empty() {
        return Err(HttpError::new(
            "Image is empty.".to_string(),
            actix_web::http::StatusCode::BAD_REQUEST,
            codes.empty,
        ));
    }
    let crop = match crop {
        [Some(x), Some(y), Some(width), Some(height)] => Some(CropRect { x, y, width, height }),
        [None, None, None, None] => None,
        _ => return Err(invalid_image(codes.bad_crop, "The crop area is incomplete.")),
    };
    Ok(ImageUpload { bytes, crop })
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

fn normalize_square(upload: ImageUpload, codes: &ImageCodes) -> Result<Vec<u8>, HttpError> {
    let img = decode_image(&upload.bytes, codes)?;
    let (w, h) = (img.width(), img.height());
    if w < MIN_AVATAR_DIMENSION || h < MIN_AVATAR_DIMENSION {
        return Err(invalid_image(
            codes.too_small,
            "Image must be at least 128×128 pixels.",
        ));
    }
    let crop = resolve_crop(w, h, upload.crop, 1, 1, codes)?;
    let img = img
        .crop_imm(crop.x, crop.y, crop.width, crop.height)
        .resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);
    encode_png(&img, codes)
}

fn normalize_animated_avatar(upload: ImageUpload) -> Result<NormalizedImage, HttpError> {
    use image::AnimationDecoder;
    use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};

    let codes = &AVATAR_CODES;
    let mut decoder = GifDecoder::new(std::io::Cursor::new(&upload.bytes))
        .map_err(|_| invalid_image(codes.invalid, "Unreadable image."))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    decoder
        .set_limits(limits)
        .map_err(|_| invalid_image(codes.invalid, "Image is too large to process."))?;
    let (w, h) = decoder.dimensions();
    if w < MIN_AVATAR_DIMENSION || h < MIN_AVATAR_DIMENSION {
        return Err(invalid_image(
            codes.too_small,
            "Image must be at least 128×128 pixels.",
        ));
    }
    let crop = resolve_crop(w, h, upload.crop, 1, 1, codes)?;

    let mut frames = Vec::new();
    for frame in decoder.into_frames() {
        let frame = frame.map_err(|_| invalid_image(codes.invalid, "Invalid image."))?;
        if frames.len() >= MAX_ANIMATION_FRAMES {
            return Err(invalid_image(
                "AVATAR_TOO_MANY_FRAMES",
                "The animation has too many frames.",
            ));
        }
        let delay = frame.delay();
        let resized = image::DynamicImage::ImageRgba8(frame.into_buffer())
            .crop_imm(crop.x, crop.y, crop.width, crop.height)
            .resize_exact(ANIMATED_AVATAR_SIZE, ANIMATED_AVATAR_SIZE, FilterType::Triangle)
            .into_rgba8();
        frames.push(image::Frame::from_parts(resized, 0, 0, delay));
    }
    if frames.is_empty() {
        return Err(invalid_image(codes.invalid, "Invalid image."));
    }

    let mut samples = Vec::new();
    for index in [0, frames.len() / 2] {
        let still = image::DynamicImage::ImageRgba8(frames[index].buffer().clone());
        samples.push(encode_png(&still, codes)?);
    }

    let mut out = Vec::new();
    {
        let mut encoder = GifEncoder::new_with_speed(&mut out, 10);
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|_| invalid_image(codes.invalid, "Failed to process image."))?;
        encoder
            .encode_frames(frames)
            .map_err(|_| invalid_image(codes.invalid, "Failed to process image."))?;
    }
    Ok(NormalizedImage {
        bytes: out,
        extension: "gif",
        content_type: "image/gif",
        moderation_samples: samples,
    })
}

pub(crate) async fn acquire_upload_permit(
    app_state: &AppState,
) -> Result<OwnedSemaphorePermit, HttpError> {
    app_state
        .upload_semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| HttpError::bad_request("upload capacity unavailable".to_string()))
}

async fn read_upload(
    app_state: &AppState,
    payload: &mut Multipart,
) -> Result<(OwnedSemaphorePermit, ImageUpload), HttpError> {
    let permit = acquire_upload_permit(app_state).await?;
    let upload = read_image_upload(payload, MAX_AVATAR_BYTES, &AVATAR_CODES).await?;
    Ok((permit, upload))
}

async fn process<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, HttpError> + Send + 'static,
) -> Result<T, HttpError> {
    web::block(job)
        .await
        .map_err(|_| HttpError::bad_request("image processing failed".to_string()))?
}

pub(crate) fn image_response(
    stream: media::ports::file_storage::ByteStream,
    content_type: &'static str,
) -> HttpResponse {
    let body = stream.map(|chunk| chunk.map(web::Bytes::from));
    HttpResponse::Ok()
        .content_type(content_type)
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
    let (_permit, upload) = read_upload(&app_state, &mut payload).await?;
    let normalized = if is_gif(&upload.bytes) {
        app_state
            .update_user_banner_usecase
            .ensure_allowed(&user_id)
            .await?;
        process(move || normalize_animated_avatar(upload)).await?
    } else {
        process(move || normalize_square(upload, &AVATAR_CODES).map(NormalizedImage::png)).await?
    };
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
    let (stream, content_type) = app_state.get_user_avatar_usecase.execute(user_id).await?;
    Ok(image_response(stream, content_type))
}

#[post("/{community_id}/logo")]
async fn upload_community_logo(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    mut payload: Multipart,
) -> Result<impl Responder, HttpError> {
    let community_id = CommunityId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);
    let (_permit, upload) = read_upload(&app_state, &mut payload).await?;
    let normalized = process(move || normalize_square(upload, &AVATAR_CODES)).await?;
    app_state
        .update_community_logo_usecase
        .execute(community_id, user_id, Some(normalized))
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{community_id}/logo")]
async fn delete_community_logo(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let community_id = CommunityId::new(path.into_inner());
    let user_id = UserId::new(auth_token.id);
    app_state
        .update_community_logo_usecase
        .execute(community_id, user_id, None)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{community_id}/logo")]
async fn get_community_logo(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    check_image_read_limit(&app_state, auth_token.id).await?;
    let community_id = CommunityId::new(path.into_inner());
    let stream = app_state.get_community_logo_usecase.execute(community_id).await?;
    Ok(image_response(stream, "image/png"))
}

#[cfg(test)]
mod tests {
    use super::{
        AVATAR_CODES, CropRect, ImageUpload, is_gif, normalize_animated_avatar, normalize_square,
        resolve_crop,
    };
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, DynamicImage, Frame, ImageFormat, RgbaImage};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        DynamicImage::ImageRgba8(RgbaImage::new(width, height))
            .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    fn gif(width: u32, height: u32, frames: usize) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut out);
            encoder.set_repeat(Repeat::Infinite).unwrap();
            for index in 0..frames {
                let shade = (index * 40 % 255) as u8;
                let buffer = RgbaImage::from_pixel(width, height, image::Rgba([shade, 0, 0, 255]));
                encoder
                    .encode_frame(Frame::from_parts(buffer, 0, 0, Delay::from_numer_denom_ms(80, 1)))
                    .unwrap();
            }
        }
        out
    }

    #[test]
    fn a_missing_area_means_the_centre_of_the_picture() {
        let crop = resolve_crop(400, 200, None, 1, 1, &AVATAR_CODES).unwrap();
        assert_eq!(crop, CropRect { x: 100, y: 0, width: 200, height: 200 });
    }

    #[test]
    fn a_square_area_inside_the_picture_is_kept_as_is() {
        let wanted = CropRect { x: 10, y: 20, width: 150, height: 150 };
        assert_eq!(resolve_crop(400, 300, Some(wanted), 1, 1, &AVATAR_CODES).unwrap(), wanted);
    }

    #[test]
    fn an_area_that_leaves_the_picture_or_is_not_square_is_refused() {
        let outside = CropRect { x: 300, y: 0, width: 200, height: 200 };
        let wide = CropRect { x: 0, y: 0, width: 300, height: 150 };
        let empty = CropRect { x: 0, y: 0, width: 0, height: 0 };
        assert!(resolve_crop(400, 300, Some(outside), 1, 1, &AVATAR_CODES).is_err());
        assert!(resolve_crop(400, 300, Some(wide), 1, 1, &AVATAR_CODES).is_err());
        assert!(resolve_crop(400, 300, Some(empty), 1, 1, &AVATAR_CODES).is_err());
    }

    #[test]
    fn a_still_avatar_comes_out_as_a_512_square() {
        let upload = ImageUpload {
            bytes: png(600, 400),
            crop: Some(CropRect { x: 100, y: 50, width: 300, height: 300 }),
        };
        let out = normalize_square(upload, &AVATAR_CODES).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert_eq!((img.width(), img.height()), (512, 512));
    }

    #[test]
    fn an_animated_avatar_keeps_its_frames() {
        let source = gif(300, 200, 3);
        assert!(is_gif(&source));
        let normalized = normalize_animated_avatar(ImageUpload { bytes: source, crop: None }).unwrap();
        assert_eq!(normalized.extension, "gif");
        assert_eq!(normalized.content_type, "image/gif");
        assert_eq!(normalized.moderation_samples.len(), 2);

        use image::AnimationDecoder;
        let decoder =
            image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&normalized.bytes)).unwrap();
        let frames = decoder.into_frames().collect_frames().unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].buffer().dimensions(), (256, 256));
    }

    #[test]
    fn a_tiny_animation_is_refused() {
        assert!(normalize_animated_avatar(ImageUpload { bytes: gif(64, 64, 2), crop: None }).is_err());
    }
}
