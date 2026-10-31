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
    let resized = img.resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);
    let rgba = resized.to_rgba8();

    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), AVATAR_SIZE, AVATAR_SIZE);
    let encoded = encoder.encode(WEBP_QUALITY);
    Ok(encoded.to_vec())
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

    #[test]
    fn reject_empty_and_huge() {
        assert!(process_avatar(&[]).is_err());
        assert!(process_avatar(&vec![0u8; MAX_BYTES + 1]).is_err());
    }

    #[test]
    fn accept_content_types() {
        assert!(accept_content_type(Some("image/png")));
        assert!(accept_content_type(Some("IMAGE/JPEG")));
        assert!(!accept_content_type(Some("text/plain")));
        assert!(accept_content_type(None));
    }
}
