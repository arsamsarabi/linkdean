use crate::{Error, Result};
use image::imageops::FilterType;
use image::ImageFormat;

const AVATAR_SIZE: u32 = 256;
const MAX_BYTES: usize = 5 * 1024 * 1024;
const WEBP_QUALITY: f32 = 90.0;

/// Decode uploaded image, resize to 256×256, encode WebP q90.
pub fn process_avatar(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.is_empty() {
        return Err(Error::BadRequest("empty image".into()));
    }
    if bytes.len() > MAX_BYTES {
        return Err(Error::BadRequest("image too large (max 5 MiB)".into()));
    }

    let img = decode_upload(bytes)?;
    let resized = img.resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);
    let rgba = resized.to_rgba8();

    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), AVATAR_SIZE, AVATAR_SIZE);
    let encoded = encoder.encode(WEBP_QUALITY);
    Ok(encoded.to_vec())
}

/// Most pixels in an uploaded image (40 megapixels). Thin strips stay allowed.
pub(crate) const MAX_PIXELS: u64 = 40_000_000;
/// Most memory one decode may allocate. A few compressed megabytes can describe an image
/// that needs gigabytes once decoded.
pub(crate) const MAX_DECODE_BYTES: u64 = 160 * 1024 * 1024;
/// Decodes running at once, across avatars and previews.
const MAX_CONCURRENT_DECODES: usize = 2;

static DECODES: (std::sync::Mutex<usize>, std::sync::Condvar) =
    (std::sync::Mutex::new(0), std::sync::Condvar::new());

/// Holds one of the decode slots until dropped.
struct DecodeSlot;

impl DecodeSlot {
    fn acquire() -> Self {
        let (count, freed) = &DECODES;
        let mut n = count.lock().unwrap_or_else(|e| e.into_inner());
        while *n >= MAX_CONCURRENT_DECODES {
            n = freed.wait(n).unwrap_or_else(|e| e.into_inner());
        }
        *n += 1;
        DecodeSlot
    }
}

impl Drop for DecodeSlot {
    fn drop(&mut self) {
        let (count, freed) = &DECODES;
        let mut n = count.lock().unwrap_or_else(|e| e.into_inner());
        *n -= 1;
        freed.notify_one();
    }
}

fn limited_reader(bytes: &[u8], format: ImageFormat) -> image::ImageReader<std::io::Cursor<&[u8]>> {
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader
}

/// Decodes an uploaded JPEG, PNG, GIF or WebP. The header's size is checked before any
/// pixels are decoded, and decodes queue for one of a few slots.
pub(crate) fn decode_upload(bytes: &[u8]) -> Result<image::DynamicImage> {
    let format = image::guess_format(bytes).map_err(|e| Error::BadRequest(e.to_string()))?;
    match format {
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Gif | ImageFormat::WebP => {}
        _ => {
            return Err(Error::BadRequest(
                "unsupported image type; use jpeg, png, webp, or gif".into(),
            ))
        }
    }
    let reader = limited_reader(bytes, format);
    let (w, h) = reader
        .into_dimensions()
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    if w as u64 * h as u64 > MAX_PIXELS {
        return Err(Error::BadRequest(
            "image too large (max 40 megapixels)".into(),
        ));
    }

    let _slot = DecodeSlot::acquire();
    let reader = limited_reader(bytes, format);
    reader.decode().map_err(|e| match e {
        image::ImageError::Limits(_) => {
            Error::BadRequest("image too large to process; use a smaller one".into())
        }
        e => Error::BadRequest(e.to_string()),
    })
}

/// Validate content-type header loosely (multipart may lie; bytes still checked).
pub fn accept_content_type(ct: Option<&str>) -> bool {
    let Some(ct) = ct else {
        return true;
    };
    let ct = ct.to_ascii_lowercase();
    ct.starts_with("image/jpeg")
        || ct.starts_with("image/jpg")
        || ct.starts_with("image/png")
        || ct.starts_with("image/webp")
        || ct.starts_with("image/gif")
        || ct.starts_with("application/octet-stream")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder as _;

    #[test]
    fn reject_empty_and_huge() {
        assert!(process_avatar(&[]).is_err());
        assert!(process_avatar(&vec![0u8; MAX_BYTES + 1]).is_err());
    }

    #[test]
    fn huge_dimensions_are_refused_before_decoding() {
        // A valid PNG header claiming 20000×20000, with no pixel data behind it.
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&[0u8; 4], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        // Patch IHDR width and height (bytes 16..24), then fix its CRC.
        png[16..20].copy_from_slice(&20_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&20_000u32.to_be_bytes());
        let crc = crc32(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        let err = decode_upload(&png).unwrap_err();
        assert!(err.to_string().contains("too large"), "{err}");
    }

    #[test]
    fn small_images_decode() {
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&[255u8; 16], 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        assert_eq!(decode_upload(&png).unwrap().width(), 2);
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn accept_content_types() {
        assert!(accept_content_type(Some("image/png")));
        assert!(accept_content_type(Some("IMAGE/JPEG")));
        assert!(!accept_content_type(Some("text/plain")));
        assert!(accept_content_type(None));
    }
}
