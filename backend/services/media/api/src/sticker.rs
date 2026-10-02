use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, delete, get, patch, post, put, web};
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::{AuthToken, RateLimitedAuthToken};
use domain::user::id::UserId;
use futures::StreamExt;
use image::imageops::FilterType;
use media::usecases::sticker::StickerFile;
use media_domain::sticker::{StickerPack, StickerRef};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::avatar::{
    ImageCodes, acquire_upload_permit, check_image_read_limit, decode_image, encode_png,
    invalid_image, is_gif,
};
use crate::http_errors::HttpError;
use crate::state::AppState;

pub const MAX_STICKER_UPLOAD_BYTES: usize = 4 * 1024 * 1024;
const STATIC_STICKER_SIZE: u32 = 512;
const ANIMATED_STICKER_SIZE: u32 = 320;
const MIN_STICKER_SIDE: u32 = 64;
const MAX_STATIC_STICKER_BYTES: usize = 512 * 1024;
const MAX_ANIMATED_STICKER_BYTES: usize = 2 * 1024 * 1024;
const MAX_STICKER_FRAMES: usize = 240;
const SHEET_COLUMNS: usize = 2;
const SHEET_TILES: usize = 4;
const MAX_DECODE_DIMENSION: u32 = 4096;
const MAX_DECODE_ALLOC: u64 = 64 * 1024 * 1024;

const STICKER_CODES: ImageCodes = ImageCodes {
    too_large: "STICKER_TOO_LARGE",
    empty: "STICKER_EMPTY",
    invalid: "STICKER_INVALID_IMAGE",
    type_not_allowed: "STICKER_TYPE_NOT_ALLOWED",
    too_small: "STICKER_TOO_SMALL",
    bad_crop: "STICKER_INVALID_IMAGE",
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StickerResponse {
    id: Uuid,
    pack_id: Uuid,
    emoji: String,
    animated: bool,
    width: i32,
    height: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackResponse {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    slug: String,
    install_count: i32,
    sticker_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed: Option<bool>,
    stickers: Vec<StickerResponse>,
}

impl From<StickerPack> for PackResponse {
    fn from(pack: StickerPack) -> Self {
        Self {
            id: pack.id,
            owner_id: pack.owner_id.uuid(),
            name: pack.name,
            slug: pack.slug,
            install_count: pack.install_count,
            sticker_count: pack.sticker_count,
            installed: pack.installed,
            stickers: pack
                .stickers
                .into_iter()
                .map(|s| StickerResponse {
                    id: s.id,
                    pack_id: s.pack_id,
                    emoji: s.emoji,
                    animated: s.animated,
                    width: s.width,
                    height: s.height,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StickerInfoResponse {
    id: Uuid,
    pack_id: Uuid,
    pack_name: String,
    pack_slug: String,
    emoji: String,
    animated: bool,
    width: i32,
    height: i32,
    deleted: bool,
}

impl From<StickerRef> for StickerInfoResponse {
    fn from(r: StickerRef) -> Self {
        Self {
            id: r.sticker.id,
            pack_id: r.sticker.pack_id,
            pack_name: r.pack_name,
            pack_slug: r.pack_slug,
            emoji: r.sticker.emoji,
            animated: r.sticker.animated,
            width: r.sticker.width,
            height: r.sticker.height,
            deleted: r.deleted,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    #[serde(default)]
    query: String,
    #[serde(default)]
    page: i64,
}

#[get("")]
async fn search_packs(
    _auth: AuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> Result<impl Responder, HttpError> {
    let query = query.into_inner();
    let packs = app_state
        .sticker_packs
        .search(&query.query, query.page)
        .await?;
    let packs: Vec<PackResponse> = packs.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(ApiResponse::success(packs)))
}

#[get("/mine")]
async fn my_packs(
    auth: AuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let packs = app_state
        .sticker_packs
        .my_packs(UserId::new(auth.id))
        .await?;
    let packs: Vec<PackResponse> = packs.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(ApiResponse::success(packs)))
}

#[get("/{pack}")]
async fn get_pack(
    _auth: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, HttpError> {
    let pack = app_state.sticker_packs.pack(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(PackResponse::from(pack))))
}

#[derive(Debug, Deserialize)]
struct CreatePackBody {
    name: String,
    #[serde(default)]
    slug: Option<String>,
}

#[post("")]
async fn create_pack(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<CreatePackBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let pack = app_state
        .sticker_packs
        .create_pack(UserId::new(auth.id), &body.name, body.slug.as_deref())
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(PackResponse::from(pack))))
}

#[derive(Debug, Deserialize)]
struct RenamePackBody {
    name: String,
}

#[patch("/{pack_id}")]
async fn rename_pack(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<RenamePackBody>,
) -> Result<impl Responder, HttpError> {
    app_state
        .sticker_packs
        .rename_pack(UserId::new(auth.id), path.into_inner(), &body.name)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{pack_id}")]
async fn delete_pack(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    app_state
        .sticker_packs
        .delete_pack(UserId::new(auth.id), path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{pack_id}/stickers")]
async fn add_sticker(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    mut payload: Multipart,
) -> Result<impl Responder, HttpError> {
    let _permit = acquire_upload_permit(&app_state).await?;
    let (bytes, emoji) = read_sticker_upload(&mut payload).await?;
    let file = web::block(move || normalize_sticker(&bytes))
        .await
        .map_err(|_| HttpError::bad_request("sticker processing failed".to_string()))??;
    let pack = app_state
        .sticker_packs
        .add_sticker(UserId::new(auth.id), path.into_inner(), &emoji, file)
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(PackResponse::from(pack))))
}

#[delete("/{pack_id}/stickers/{sticker_id}")]
async fn delete_sticker(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (pack_id, sticker_id) = path.into_inner();
    app_state
        .sticker_packs
        .delete_sticker(UserId::new(auth.id), pack_id, sticker_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReorderBody {
    sticker_ids: Vec<Uuid>,
}

#[put("/{pack_id}/order")]
async fn reorder_stickers(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<ReorderBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    if body.sticker_ids.len() > media_domain::sticker::MAX_STICKERS_PER_PACK as usize {
        return Err(HttpError::bad_request("too many stickers".to_string()));
    }
    app_state
        .sticker_packs
        .reorder(UserId::new(auth.id), path.into_inner(), &body.sticker_ids)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[put("/{pack_id}/install")]
async fn install_pack(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    app_state
        .sticker_packs
        .install(UserId::new(auth.id), path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{pack_id}/install")]
async fn uninstall_pack(
    auth: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    app_state
        .sticker_packs
        .uninstall(UserId::new(auth.id), path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{sticker_id}/file")]
async fn sticker_file(
    auth: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    check_image_read_limit(&app_state, auth.id).await?;
    let (stream, content_type) = app_state
        .sticker_packs
        .sticker_file(path.into_inner())
        .await?;
    let body = stream.map(|chunk| chunk.map(web::Bytes::from));
    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .insert_header(("X-Content-Type-Options", "nosniff"))
        .insert_header(("Cache-Control", "private, max-age=31536000, immutable"))
        .streaming(body))
}

#[get("/stickers/{sticker_id}")]
async fn internal_sticker(
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    match app_state.sticker_packs.claim_for_message(path.into_inner()).await {
        Ok(sticker) => Ok(HttpResponse::Ok().json(StickerInfoResponse::from(sticker))),
        Err(media::usecases::sticker::StickerError::StickerNotFound) => {
            Ok(HttpResponse::NotFound().finish())
        }
        Err(e) => Err(e.into()),
    }
}

pub fn sticker_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/sticker-packs")
            .service(search_packs)
            .service(create_pack)
            .service(my_packs)
            .service(get_pack)
            .service(rename_pack)
            .service(delete_pack)
            .service(add_sticker)
            .service(delete_sticker)
            .service(reorder_stickers)
            .service(install_pack)
            .service(uninstall_pack),
    )
    .service(web::scope("/v1/stickers").service(sticker_file));
}

pub fn internal_sticker_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(internal_sticker);
}

async fn read_sticker_upload(payload: &mut Multipart) -> Result<(Vec<u8>, String), HttpError> {
    let mut bytes: Option<Vec<u8>> = None;
    let mut emoji: Option<String> = None;
    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| HttpError::bad_request(e.to_string()))?;
        let is_file = match field.name() {
            Some("file") => true,
            Some("emoji") => false,
            _ => continue,
        };
        let limit = if is_file { MAX_STICKER_UPLOAD_BYTES } else { 64 };
        let mut buffer = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| HttpError::bad_request(e.to_string()))?;
            if buffer.len() + chunk.len() > limit {
                return Err(if is_file {
                    HttpError::new(
                        "Sticker file is too large.".to_string(),
                        actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
                        STICKER_CODES.too_large,
                    )
                } else {
                    invalid_image("STICKER_EMOJI_INVALID", "The sticker emoji is too long.")
                });
            }
            buffer.extend_from_slice(&chunk);
        }
        if is_file {
            bytes = Some(buffer);
        } else {
            emoji = Some(
                String::from_utf8(buffer)
                    .map_err(|_| invalid_image("STICKER_EMOJI_INVALID", "The sticker emoji is invalid."))?,
            );
        }
    }
    let bytes = bytes.ok_or_else(|| HttpError::bad_request("missing file field".to_string()))?;
    if bytes.is_empty() {
        return Err(invalid_image(STICKER_CODES.empty, "Sticker file is empty."));
    }
    let emoji = emoji.ok_or_else(|| invalid_image("STICKER_EMOJI_INVALID", "Pick an emoji for the sticker."))?;
    Ok((bytes, emoji))
}

pub(crate) fn fit_within(width: u32, height: u32, side: u32) -> (u32, u32) {
    if width >= height {
        let h = ((u64::from(height) * u64::from(side)) / u64::from(width.max(1))).max(1) as u32;
        (side, h)
    } else {
        let w = ((u64::from(width) * u64::from(side)) / u64::from(height.max(1))).max(1) as u32;
        (w, side)
    }
}

fn normalize_sticker(bytes: &[u8]) -> Result<StickerFile, HttpError> {
    if is_gif(bytes) {
        return normalize_animated_sticker(bytes);
    }
    let codes = &STICKER_CODES;
    let img = decode_image(bytes, codes)?;
    if img.width() < MIN_STICKER_SIDE || img.height() < MIN_STICKER_SIDE {
        return Err(invalid_image(codes.too_small, "Stickers must be at least 64×64 pixels."));
    }
    let (w, h) = fit_within(img.width(), img.height(), STATIC_STICKER_SIZE);
    let img = image::DynamicImage::ImageRgba8(img.resize_exact(w, h, FilterType::Lanczos3).into_rgba8());
    let png = encode_png(&img, codes)?;
    if png.len() > MAX_STATIC_STICKER_BYTES {
        return Err(HttpError::new(
            "The sticker is over 512 KB after processing.".to_string(),
            actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
            codes.too_large,
        ));
    }
    Ok(StickerFile {
        bytes: png,
        animated: false,
        width: w,
        height: h,
        moderation_samples: Vec::new(),
    })
}

#[derive(Default)]
struct SheetBuilder {
    last: Option<image::RgbaImage>,
    tiles: Vec<image::RgbaImage>,
}

impl SheetBuilder {
    fn push(&mut self, frame: &image::RgbaImage) -> Option<image::RgbaImage> {
        if self.last.as_ref() == Some(frame) {
            return None;
        }
        self.last = Some(frame.clone());
        self.tiles.push(frame.clone());
        if self.tiles.len() < SHEET_TILES {
            return None;
        }
        self.flush()
    }

    fn flush(&mut self) -> Option<image::RgbaImage> {
        let first = self.tiles.first()?;
        let (w, h) = first.dimensions();
        let columns = self.tiles.len().min(SHEET_COLUMNS) as u32;
        let rows = self.tiles.len().div_ceil(SHEET_COLUMNS) as u32;
        let mut sheet = image::RgbaImage::new(w * columns, h * rows);
        for (index, tile) in self.tiles.iter().enumerate() {
            let x = (index % SHEET_COLUMNS) as i64 * i64::from(w);
            let y = (index / SHEET_COLUMNS) as i64 * i64::from(h);
            image::imageops::overlay(&mut sheet, tile, x, y);
        }
        self.tiles.clear();
        Some(sheet)
    }
}

fn normalize_animated_sticker(bytes: &[u8]) -> Result<StickerFile, HttpError> {
    use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
    use image::{AnimationDecoder, ImageDecoder};

    let codes = &STICKER_CODES;
    let mut decoder = GifDecoder::new(std::io::Cursor::new(bytes))
        .map_err(|_| invalid_image(codes.invalid, "Unreadable sticker."))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    decoder
        .set_limits(limits)
        .map_err(|_| invalid_image(codes.invalid, "Sticker is too large to process."))?;
    let (width, height) = decoder.dimensions();
    if width < MIN_STICKER_SIDE || height < MIN_STICKER_SIDE {
        return Err(invalid_image(codes.too_small, "Stickers must be at least 64×64 pixels."));
    }
    let (w, h) = fit_within(width, height, ANIMATED_STICKER_SIZE);
    let failed = || invalid_image(codes.invalid, "Failed to process sticker.");
    let sample = |sheet: image::RgbaImage| encode_png(&image::DynamicImage::ImageRgba8(sheet), codes);
    let mut sheets = SheetBuilder::default();
    let mut samples = Vec::new();
    let mut count = 0usize;
    let mut out = Vec::new();
    {
        let mut encoder = GifEncoder::new_with_speed(&mut out, 10);
        encoder.set_repeat(Repeat::Infinite).map_err(|_| failed())?;
        for frame in decoder.into_frames() {
            let frame = frame.map_err(|_| invalid_image(codes.invalid, "Invalid sticker."))?;
            if count >= MAX_STICKER_FRAMES {
                return Err(invalid_image("STICKER_TOO_MANY_FRAMES", "The animation has too many frames."));
            }
            count += 1;
            let delay = frame.delay();
            let resized = image::DynamicImage::ImageRgba8(frame.into_buffer())
                .resize_exact(w, h, FilterType::Triangle)
                .into_rgba8();
            if let Some(sheet) = sheets.push(&resized) {
                samples.push(sample(sheet)?);
            }
            encoder
                .encode_frame(image::Frame::from_parts(resized, 0, 0, delay))
                .map_err(|_| failed())?;
        }
    }
    if count == 0 {
        return Err(invalid_image(codes.invalid, "Invalid sticker."));
    }
    if let Some(sheet) = sheets.flush() {
        samples.push(sample(sheet)?);
    }
    if out.len() > MAX_ANIMATED_STICKER_BYTES {
        return Err(HttpError::new(
            "The animated sticker is over 2 MB after processing.".to_string(),
            actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
            codes.too_large,
        ));
    }
    Ok(StickerFile {
        bytes: out,
        animated: true,
        width: w,
        height: h,
        moderation_samples: samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let img = image::DynamicImage::new_rgba8(width, height);
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    fn gif(width: u32, height: u32, frames: usize) -> Vec<u8> {
        use image::codecs::gif::{GifEncoder, Repeat};
        let mut out = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut out);
            encoder.set_repeat(Repeat::Infinite).unwrap();
            let frames: Vec<image::Frame> = (0..frames)
                .map(|i| {
                    let mut buf = image::RgbaImage::new(width, height);
                    buf.put_pixel(0, 0, image::Rgba([i as u8, 0, 0, 255]));
                    image::Frame::new(buf)
                })
                .collect();
            encoder.encode_frames(frames).unwrap();
        }
        out
    }

    #[test]
    fn fits_the_longer_side_and_keeps_the_aspect() {
        assert_eq!(fit_within(1024, 512, 512), (512, 256));
        assert_eq!(fit_within(100, 400, 512), (128, 512));
        assert_eq!(fit_within(200, 200, 320), (320, 320));
    }

    #[test]
    fn static_stickers_become_png_within_512() {
        let file = normalize_sticker(&png(1000, 500)).unwrap();
        assert!(!file.animated);
        assert_eq!((file.width, file.height), (512, 256));
        assert!(file.bytes.starts_with(b"\x89PNG"));
    }

    #[test]
    fn gif_stickers_stay_animated_within_320() {
        let file = normalize_sticker(&gif(400, 200, 3)).unwrap();
        assert!(file.animated);
        assert_eq!((file.width, file.height), (320, 160));
        assert_eq!(file.moderation_samples.len(), 1);
        assert!(is_gif(&file.bytes));
    }

    #[test]
    fn partial_gif_frames_are_composited_before_resizing() {
        use image::AnimationDecoder;
        let mut bytes = Vec::new();
        {
            let palette = [255, 0, 0, 0, 0, 255];
            let mut encoder = gif::Encoder::new(&mut bytes, 160, 160, &palette).unwrap();
            encoder.set_repeat(gif::Repeat::Infinite).unwrap();
            let mut full = gif::Frame::from_indexed_pixels(160, 160, vec![0; 160 * 160], None);
            full.delay = 10;
            encoder.write_frame(&full).unwrap();
            let mut patch = gif::Frame::from_indexed_pixels(16, 16, vec![1; 16 * 16], None);
            patch.left = 80;
            patch.top = 80;
            patch.delay = 10;
            encoder.write_frame(&patch).unwrap();
        }
        let file = normalize_sticker(&bytes).unwrap();
        let frames = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(file.bytes))
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        assert_eq!(frames.len(), 2);
        let second = frames[1].buffer();
        assert_eq!(second.dimensions(), (320, 320));
        let corner = second.get_pixel(10, 10);
        assert!(corner[0] > 200 && corner[2] < 60, "{corner:?}");
        let patch = second.get_pixel(176, 176);
        assert!(patch[2] > 200 && patch[0] < 60, "{patch:?}");
    }

    #[test]
    fn every_distinct_frame_lands_on_a_moderation_sheet() {
        let frame = |shade: u8| image::Frame::new(image::RgbaImage::from_pixel(10, 10, image::Rgba([shade, 0, 0, 255])));
        let frames: Vec<image::Frame> = [1, 1, 2, 3, 4, 5, 6].into_iter().map(frame).collect();
        let mut builder = SheetBuilder::default();
        let mut sheets: Vec<image::RgbaImage> = frames.iter().filter_map(|f| builder.push(f.buffer())).collect();
        sheets.extend(builder.flush());
        assert_eq!(sheets.len(), 2);
        assert_eq!(sheets[0].dimensions(), (20, 20));
        assert_eq!(sheets[0].get_pixel(15, 15)[0], 4);
        assert_eq!(sheets[1].dimensions(), (20, 10));
        assert_eq!(sheets[1].get_pixel(15, 5)[0], 6);
    }

    #[test]
    fn tiny_or_broken_files_are_rejected() {
        assert!(normalize_sticker(&png(32, 32)).is_err());
        assert!(normalize_sticker(b"not an image").is_err());
    }
}
