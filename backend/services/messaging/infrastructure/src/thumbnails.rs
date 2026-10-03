use std::io::Cursor;

use async_trait::async_trait;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, imageops::FilterType, metadata::Orientation};
use messaging::ports::thumbnailer::{Thumbnail, Thumbnailer};

pub const THUMBNAIL_SIDE: u32 = 320;
const MAX_DIMENSION: u32 = 8192;
const MAX_ALLOC: u64 = 128 * 1024 * 1024;
const JPEG_QUALITY: u8 = 80;

pub struct ImageThumbnailer;

pub fn render_thumbnail(bytes: &[u8]) -> Option<Thumbnail> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().ok()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    let thumb = if image.width() > THUMBNAIL_SIDE || image.height() > THUMBNAIL_SIDE {
        image.resize(THUMBNAIL_SIDE, THUMBNAIL_SIDE, FilterType::Triangle)
    } else {
        image
    };
    let mut out = Vec::new();
    if thumb.color().has_alpha() {
        DynamicImage::ImageRgba8(thumb.into_rgba8())
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .ok()?;
        return Some(Thumbnail { bytes: out, content_type: "image/png" });
    }
    let rgb = DynamicImage::ImageRgb8(thumb.into_rgb8());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    encoder.encode_image(&rgb).ok()?;
    Some(Thumbnail { bytes: out, content_type: "image/jpeg" })
}

#[async_trait]
impl Thumbnailer for ImageThumbnailer {
    async fn thumbnail(&self, image: Vec<u8>) -> Option<Thumbnail> {
        tokio::task::spawn_blocking(move || render_thumbnail(&image)).await.ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        image.write_to(&mut Cursor::new(&mut out), format).unwrap();
        out
    }

    #[test]
    fn large_photos_shrink_to_a_jpeg_within_the_side() {
        let photo = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(1600, 900, image::Rgb([200, 10, 10])));
        let thumb = render_thumbnail(&encode(photo, ImageFormat::Png)).unwrap();
        assert_eq!(thumb.content_type, "image/jpeg");
        let decoded = image::load_from_memory(&thumb.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (320, 180));
    }

    #[test]
    fn transparent_images_stay_png_and_small_ones_keep_their_size() {
        let icon = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(64, 48, image::Rgba([0, 0, 0, 0])));
        let thumb = render_thumbnail(&encode(icon, ImageFormat::Png)).unwrap();
        assert_eq!(thumb.content_type, "image/png");
        let decoded = image::load_from_memory(&thumb.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 48));
    }

    #[test]
    fn exif_rotation_is_applied() {
        let photo = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(40, 20, image::Rgb([10, 200, 10])));
        let jpeg = encode(photo, ImageFormat::Jpeg);
        let exif: [u8; 34] = [
            0xFF, 0xE1, 0x00, 0x20, b'E', b'x', b'i', b'f', 0, 0, b'M', b'M', 0, 42, 0, 0, 0, 8, 0, 1, 0x01, 0x12,
            0, 3, 0, 0, 0, 1, 0, 6, 0, 0, 0, 0,
        ];
        let mut rotated = jpeg[..2].to_vec();
        rotated.extend_from_slice(&exif);
        rotated.extend_from_slice(&jpeg[2..]);
        let thumb = render_thumbnail(&rotated).unwrap();
        let decoded = image::load_from_memory(&thumb.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (20, 40));
    }

    #[test]
    fn non_images_have_no_thumbnail() {
        assert!(render_thumbnail(b"%PDF-1.7 not an image").is_none());
    }
}
