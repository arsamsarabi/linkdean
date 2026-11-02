use crate::{Error, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ColorType, DynamicImage, GenericImageView, ImageFormat};

/// Card-friendly 16:9 cover size (matches OG-ish ratio).
const PREVIEW_W: u32 = 1200;
const PREVIEW_H: u32 = 675;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const JPEG_QUALITY: u8 = 85;

/// Decode uploaded image, cover-crop to 1200×675, encode JPEG q85.
pub fn process_bookmark_preview(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.is_empty() {
        return Err(Error::BadRequest("empty image".into()));
    }
    if bytes.len() > MAX_BYTES {
        return Err(Error::BadRequest("image too large (max 8 MiB)".into()));
    }

    let format = image::guess_format(bytes).map_err(|e| Error::BadRequest(e.to_string()))?;
    match format {
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Gif | ImageFormat::WebP => {}
        _ => {
            return Err(Error::BadRequest(
                "unsupported image type; use jpeg, png, webp, or gif".into(),
            ))
        }
    }

    let img = image::load_from_memory(bytes).map_err(|e| Error::BadRequest(e.to_string()))?;
    let rgb = cover_crop_rgb(img, PREVIEW_W, PREVIEW_H);

    let mut out = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    encoder
        .encode(rgb.as_raw(), PREVIEW_W, PREVIEW_H, ColorType::Rgb8.into())
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    Ok(out)
}

fn cover_crop_rgb(img: DynamicImage, tw: u32, th: u32) -> image::RgbImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return image::RgbImage::new(tw, th);
    }
    // Crop before resizing. Scaling first turns a 1×65535 image into tens of gigabytes.
    let (cw, ch) = cover_crop_size(w, h, tw, th);
    let x = (w - cw) / 2;
    let y = (h - ch) / 2;
    img.crop_imm(x, y, cw, ch)
        .resize_exact(tw, th, FilterType::Lanczos3)
        .to_rgb8()
}

/// Largest centred region of a `w`×`h` image with the `tw`:`th` aspect ratio.
fn cover_crop_size(w: u32, h: u32, tw: u32, th: u32) -> (u32, u32) {
    let (w64, h64, tw64, th64) = (w as u64, h as u64, tw as u64, th as u64);
    if w64 * th64 > h64 * tw64 {
        (((h64 * tw64 / th64).max(1) as u32).min(w), h)
    } else {
        (w, ((w64 * th64 / tw64).max(1) as u32).min(h))
    }
}

/// Validate content-type header loosely (multipart may lie; bytes still checked).
pub fn accept_content_type(ct: Option<&str>) -> bool {
    crate::avatar::accept_content_type(ct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn tiny_png() -> Vec<u8> {
        let img: ImageBuffer<Rgb<u8>, _> =
            ImageBuffer::from_fn(32, 24, |x, y| Rgb([(x * 8) as u8, (y * 10) as u8, 180]));
        let mut buf = Vec::new();
        let enc = image::codecs::png::PngEncoder::new(&mut buf);
        use image::ImageEncoder;
        enc.write_image(img.as_raw(), 32, 24, ColorType::Rgb8.into())
            .expect("png");
        buf
    }

    #[test]
    fn reject_empty_and_huge() {
        assert!(process_bookmark_preview(&[]).is_err());
        assert!(process_bookmark_preview(&vec![0u8; MAX_BYTES + 1]).is_err());
    }

    #[test]
    fn encodes_jpeg_cover() {
        let jpeg = process_bookmark_preview(&tiny_png()).expect("process");
        assert!(!jpeg.is_empty());
        let fmt = image::guess_format(&jpeg).expect("guess");
        assert_eq!(fmt, ImageFormat::Jpeg);
        let decoded = image::load_from_memory(&jpeg).expect("decode");
        assert_eq!(decoded.dimensions(), (PREVIEW_W, PREVIEW_H));
    }

    #[test]
    fn extreme_aspect_ratios_stay_small() {
        for (w, h) in [(1, 65_535), (65_535, 1)] {
            let img = image::GrayImage::new(w, h);
            let mut buf = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
                .unwrap();
            let out = process_bookmark_preview(&buf).expect("preview");
            let decoded = image::load_from_memory(&out).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (PREVIEW_W, PREVIEW_H));
        }
    }

    #[test]
    fn cover_crop_keeps_the_target_aspect() {
        assert_eq!(cover_crop_size(1600, 900, 16, 9), (1600, 900));
        assert_eq!(cover_crop_size(1000, 1000, 16, 9), (1000, 562));
        assert_eq!(cover_crop_size(1000, 100, 16, 9), (177, 100));
        assert_eq!(cover_crop_size(1, 65_535, 16, 9), (1, 1));
    }
}
