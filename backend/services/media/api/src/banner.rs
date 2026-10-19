use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, delete, get, post, web};
use api_common::middleware::auth::{AuthToken, RateLimitedAuthToken};
use domain::user::id::UserId;
use image::imageops::FilterType;
use uuid::Uuid;

use crate::avatar::{
    ImageCodes, ImageUpload, check_image_read_limit, decode_image, encode_png, image_response,
    acquire_upload_permit, invalid_image, read_image_upload, resolve_crop,
};
use crate::http_errors::HttpError;
use crate::state::AppState;

pub const MAX_BANNER_BYTES: usize = 8 * 1024 * 1024;
const MIN_BANNER_WIDTH: u32 = 600;
const MIN_BANNER_HEIGHT: u32 = 200;
const BANNER_WIDTH: u32 = 1500;
const BANNER_HEIGHT: u32 = 500;

const BANNER_CODES: ImageCodes = ImageCodes {
    too_large: "BANNER_TOO_LARGE",
    empty: "BANNER_EMPTY",
    invalid: "BANNER_INVALID_IMAGE",
    type_not_allowed: "BANNER_TYPE_NOT_ALLOWED",
    too_small: "BANNER_TOO_SMALL",
    bad_crop: "BANNER_CROP_INVALID",
};

pub(crate) fn normalize_banner(upload: ImageUpload) -> Result<Vec<u8>, HttpError> {
    let img = decode_image(&upload.bytes, &BANNER_CODES)?;
    let (w, h) = (img.width(), img.height());
    if w < MIN_BANNER_WIDTH || h < MIN_BANNER_HEIGHT {
        return Err(invalid_image(
            BANNER_CODES.too_small,
            "Banner must be at least 600×200 pixels.",
        ));
    }
    let crop = resolve_crop(w, h, upload.crop, BANNER_WIDTH, BANNER_HEIGHT, &BANNER_CODES)?;
    let img = img
        .crop_imm(crop.x, crop.y, crop.width, crop.height)
        .resize_exact(BANNER_WIDTH, BANNER_HEIGHT, FilterType::Lanczos3);
    encode_png(&img, &BANNER_CODES)
}

#[post("/me/banner")]
async fn upload_my_banner(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let _permit = acquire_upload_permit(&app_state).await?;
    let upload = read_image_upload(&mut payload, MAX_BANNER_BYTES, &BANNER_CODES).await?;
    let normalized = web::block(move || normalize_banner(upload))
        .await
        .map_err(|_| HttpError::bad_request("image processing failed".to_string()))??;
    app_state
        .update_user_banner_usecase
        .execute(user_id, normalized)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/me/banner")]
async fn delete_my_banner(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    app_state
        .update_user_banner_usecase
        .remove(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{user_id}/banner")]
async fn get_user_banner(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    check_image_read_limit(&app_state, auth_token.id).await?;
    let stream = app_state
        .get_user_banner_usecase
        .execute(UserId::new(path.into_inner()))
        .await?;
    Ok(image_response(stream, "image/png"))
}

#[cfg(test)]
mod tests {
    use super::normalize_banner;
    use crate::avatar::{CropRect, ImageUpload};
    use image::{DynamicImage, ImageFormat, RgbImage};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(RgbImage::new(width, height))
            .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    fn whole(bytes: Vec<u8>) -> ImageUpload {
        ImageUpload { bytes, crop: None }
    }

    fn size_of(bytes: &[u8]) -> (u32, u32) {
        let img = image::load_from_memory(bytes).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn a_wide_picture_becomes_a_three_to_one_banner() {
        assert_eq!(size_of(&normalize_banner(whole(png(3000, 600))).unwrap()), (1500, 500));
    }

    #[test]
    fn a_tall_picture_is_cropped_to_the_same_shape() {
        assert_eq!(size_of(&normalize_banner(whole(png(900, 1200))).unwrap()), (1500, 500));
    }

    #[test]
    fn a_picture_below_the_minimum_is_refused() {
        assert!(normalize_banner(whole(png(599, 400))).is_err());
        assert!(normalize_banner(whole(png(800, 199))).is_err());
    }

    #[test]
    fn something_that_is_not_a_picture_is_refused() {
        assert!(normalize_banner(whole(b"definitely not an image".to_vec())).is_err());
    }

    #[test]
    fn a_chosen_area_is_used_when_it_has_the_right_shape() {
        let upload = ImageUpload {
            bytes: png(3000, 2000),
            crop: Some(CropRect { x: 300, y: 200, width: 1800, height: 600 }),
        };
        assert_eq!(size_of(&normalize_banner(upload).unwrap()), (1500, 500));
    }

    #[test]
    fn an_area_outside_the_picture_or_of_the_wrong_shape_is_refused() {
        let outside = ImageUpload {
            bytes: png(3000, 2000),
            crop: Some(CropRect { x: 2000, y: 0, width: 1800, height: 600 }),
        };
        let square = ImageUpload {
            bytes: png(3000, 2000),
            crop: Some(CropRect { x: 0, y: 0, width: 900, height: 900 }),
        };
        assert!(normalize_banner(outside).is_err());
        assert!(normalize_banner(square).is_err());
    }
}
