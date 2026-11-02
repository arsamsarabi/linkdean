//! Serving a bookmark's stored images: its screenshot or upload, and cached favicon and OG
//! image. Callers check access first.

use crate::error::ApiResult;
use crate::state::AppState;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use linkdean_core::Error;
use uuid::Uuid;

/// Only the owner's browser and the people they share with may cache it.
pub(super) const PRIVATE_CACHE: &str = "private, max-age=86400";
/// Anyone may cache it, briefly, so a bookmark taken off a public list stops showing soon.
pub(super) const PUBLIC_CACHE: &str = "public, max-age=300";

#[derive(Debug, Clone, Copy)]
pub(super) enum Asset {
    Preview,
    Favicon,
    Og,
}

pub(super) async fn serve(
    state: &AppState,
    id: Uuid,
    asset: Asset,
    cache: &'static str,
) -> ApiResult<Response> {
    let (path, ct_path, default_ct) = match asset {
        Asset::Preview => (state.preview_path(id), None, "image/jpeg"),
        Asset::Favicon => (
            state.favicon_path(id),
            Some(state.favicon_ct_path(id)),
            "image/png",
        ),
        Asset::Og => (state.og_path(id), Some(state.og_ct_path(id)), "image/jpeg"),
    };
    let bytes = tokio::fs::read(&path).await.map_err(|_| Error::NotFound)?;
    let stored_ct = match ct_path {
        Some(p) => tokio::fs::read_to_string(&p).await.ok(),
        None => None,
    };
    let headers = asset_headers(stored_ct.as_deref(), default_ct, cache);
    Ok((StatusCode::OK, headers, bytes).into_response())
}

fn asset_headers(
    stored_ct: Option<&str>,
    default_ct: &'static str,
    cache: &'static str,
) -> HeaderMap {
    let ct = stored_ct
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| HeaderValue::from_str(s).ok())
        .unwrap_or_else(|| HeaderValue::from_static(default_ct));
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, ct);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    // Remote content on our origin, SVG included: it may be an image, but it never gets to run.
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("sandbox; default-src 'none'; style-src 'unsafe-inline'"),
    );
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_asset_is_sandboxed_and_not_sniffed() {
        for stored in [None, Some("image/svg+xml"), Some("  "), Some("bad\nvalue")] {
            let h = asset_headers(stored, "image/png", PRIVATE_CACHE);
            assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
            assert!(h[header::CONTENT_SECURITY_POLICY]
                .to_str()
                .unwrap()
                .starts_with("sandbox;"));
        }
    }

    #[test]
    fn stored_content_type_wins_over_the_default() {
        let h = asset_headers(Some("image/svg+xml\n"), "image/png", PUBLIC_CACHE);
        assert_eq!(h[header::CONTENT_TYPE], "image/svg+xml");
        assert_eq!(h[header::CACHE_CONTROL], PUBLIC_CACHE);
        let h = asset_headers(Some(""), "image/png", PUBLIC_CACHE);
        assert_eq!(h[header::CONTENT_TYPE], "image/png");
    }
}
