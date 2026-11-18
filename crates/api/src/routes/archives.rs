//! Archived copies of bookmarks: queue them, list them, serve them, delete them.

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use linkdean_core::{
    crypto, ArchiveFormat, Bookmark, BookmarkArchiveView, BulkArchive, BulkArchiveFailure,
    BulkArchiveResult, Error, RequestArchive, MAX_BULK_ARCHIVE_IDS,
};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

/// An archived page is someone else's HTML on our origin. Scripts were stripped at capture; the
/// sandbox gives it an opaque origin as well, so it can never reach the session or the API.
const HTML_CSP: &str = "sandbox allow-popups allow-popups-to-escape-sandbox; default-src 'none'; \
                        img-src data:; style-src 'unsafe-inline' data:; font-src data:; \
                        media-src data:; frame-ancestors 'self'";
/// PDFs, images and media cannot run script when served with their sniffed type. A sandbox or
/// `default-src 'none'` would stop Chrome's PDF viewer, so these only limit who may frame them.
const INERT_CSP: &str = "frame-ancestors 'self'";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(request_one, list, delete_all))
        .routes(routes!(request_bulk))
        .routes(routes!(get_copy))
}

/// A bookmark whose copies the user may open: one they can read, or a pin on a dashboard
/// shared with them (as for preview images).
async fn readable(state: &AppState, user_id: Uuid, id: Uuid) -> Result<Bookmark, Error> {
    match db::get_bookmark_for_assets(&state.pool, user_id, id).await {
        Err(Error::NotFound) => {
            let b = db::get_bookmark_raw(&state.pool, user_id, id).await?;
            if b.is_vault {
                Ok(b)
            } else {
                Err(Error::NotFound)
            }
        }
        other => other,
    }
}

#[utoipa::path(
    post,
    operation_id = "bookmarks_request_archive",
    path = "/bookmarks/{id}/archive",
    tag = "archives",
    params(("id" = Uuid, Path)),
    request_body = RequestArchive,
    responses(
        (status = 200, description = "Queued; every copy of the bookmark", body = Vec<BookmarkArchiveView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn request_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<RequestArchive>,
) -> ApiResult<Json<Vec<BookmarkArchiveView>>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let views = db::request_archive(&state.pool, user.id, id, &req.formats, &keyring).await?;
    state.archive_wake.notify_one();
    Ok(Json(views))
}

#[utoipa::path(
    post,
    operation_id = "bookmarks_request_archive_bulk",
    path = "/bookmarks/archive",
    tag = "archives",
    request_body = BulkArchive,
    responses(
        (status = 200, description = "Queued, with the bookmarks that could not be", body = BulkArchiveResult),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn request_bulk(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Json(req): Json<BulkArchive>,
) -> ApiResult<Json<BulkArchiveResult>> {
    if req.ids.len() > MAX_BULK_ARCHIVE_IDS {
        return Err(Error::BadRequest(format!(
            "archive at most {MAX_BULK_ARCHIVE_IDS} bookmarks at once"
        ))
        .into());
    }
    let formats = linkdean_core::clean_archive_formats(&req.formats)?;
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let mut out = BulkArchiveResult {
        ok: 0,
        failed: Vec::new(),
    };
    for id in req.ids {
        match db::request_archive(&state.pool, user.id, id, &formats, &keyring).await {
            Ok(_) => out.ok += 1,
            Err(e) => out.failed.push(BulkArchiveFailure {
                id,
                error: bulk_reason(&e),
            }),
        }
    }
    if out.ok > 0 {
        state.archive_wake.notify_one();
    }
    Ok(Json(out))
}

fn bulk_reason(e: &Error) -> String {
    match e {
        Error::NotFound => "Not found.".into(),
        Error::Forbidden => "You can't change this bookmark.".into(),
        Error::VaultLocked => "Unlock its vault first.".into(),
        Error::QuotaExceeded { .. } => db::ARCHIVE_STORAGE_FULL.into(),
        Error::BadRequest(m) => m.clone(),
        _ => "Something went wrong.".into(),
    }
}

#[utoipa::path(
    get,
    operation_id = "bookmarks_list_archives",
    path = "/bookmarks/{id}/archive",
    tag = "archives",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Every copy of the bookmark, stored or on its way", body = Vec<BookmarkArchiveView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<BookmarkArchiveView>>> {
    readable(&state, user.id, id).await?;
    Ok(Json(db::list_archives(&state.pool, id).await?))
}

#[utoipa::path(
    delete,
    operation_id = "bookmarks_delete_archives",
    path = "/bookmarks/{id}/archive",
    tag = "archives",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Every copy deleted", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_all(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let names = db::delete_archives(&state.pool, user.id, id).await?;
    crate::archive_runner::remove_files(&state.archive_dir(id), names.iter()).await;
    let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    if let Some(view) = db::bookmark_to_view(
        &state.pool,
        raw,
        &linkdean_core::VaultKeyring::default(),
        user.id,
    )
    .await?
    {
        if !view.is_vault {
            if let Err(e) = crate::search_index::upsert(&state, &view).await {
                tracing::warn!(bookmark_id = %id, error = %e, "meili upsert after archive delete failed");
            }
        }
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct CopyQuery {
    /// `1` serves the copy as a download.
    download: Option<String>,
}

#[utoipa::path(
    get,
    operation_id = "bookmarks_get_archive",
    path = "/bookmarks/{id}/archive/{format}",
    tag = "archives",
    params(("id" = Uuid, Path), ("format" = ArchiveFormat, Path), CopyQuery),
    responses(
        (status = 200, description = "The copy: sandboxed HTML, a PDF, or the original file"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn get_copy(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path((id, format)): Path<(Uuid, String)>,
    Query(q): Query<CopyQuery>,
) -> ApiResult<Response> {
    let format = ArchiveFormat::parse(&format).map_err(|_| Error::NotFound)?;
    let b = readable(&state, user.id, id).await?;
    let row = db::archive_row(&state.pool, id, format)
        .await?
        .ok_or(Error::NotFound)?;
    let name = row.file_name.clone().ok_or(Error::NotFound)?;
    let stored = tokio::fs::read(state.archive_dir(id).join(&name))
        .await
        .map_err(|_| Error::NotFound)?;
    let bytes = match (row.key_vault_id, &row.nonce) {
        (Some(vault_id), Some(nonce)) => {
            let keyring = state.vault_keyring(&vault_session, user.id).await;
            let key = keyring.get(vault_id).ok_or(Error::VaultLocked)?;
            crypto::decrypt_archive(key, vault_id, id, format, &stored, nonce)?
        }
        _ => stored,
    };
    let download = q.download.is_some_and(|v| v != "0" && v != "false");
    let host = row
        .final_url
        .as_deref()
        .or(Some(b.url.as_str()).filter(|u| !u.is_empty()))
        .and_then(|u| url::Url::parse(u).ok())
        .and_then(|u| u.host_str().map(str::to_string));
    let headers = copy_headers(
        format,
        row.content_type.as_deref().unwrap_or(""),
        &bytes,
        download,
        host.as_deref(),
        row.captured_at,
    );
    Ok((StatusCode::OK, headers, bytes).into_response())
}

fn copy_headers(
    format: ArchiveFormat,
    content_type: &str,
    bytes: &[u8],
    download: bool,
    host: Option<&str>,
    captured_at: Option<DateTime<Utc>>,
) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("SAMEORIGIN"),
    );
    let inline = match format {
        ArchiveFormat::Html => Some("text/html; charset=utf-8"),
        _ => linkdean_worker::archive::inline_type(content_type, bytes),
    };
    let csp = if format == ArchiveFormat::Html {
        HTML_CSP
    } else {
        INERT_CSP
    };
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(csp),
    );
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(inline.unwrap_or("application/octet-stream")),
    );
    if download || inline.is_none() {
        let name = download_name(format, inline.unwrap_or(content_type), host, captured_at);
        if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
            h.insert(header::CONTENT_DISPOSITION, v);
        }
    }
    h
}

/// `<host>-<YYYY-MM-DD>.<ext>`, safe to put in a header.
fn download_name(
    format: ArchiveFormat,
    content_type: &str,
    host: Option<&str>,
    captured_at: Option<DateTime<Utc>>,
) -> String {
    let host: String = host
        .unwrap_or("archive")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
        .collect();
    let host = if host.is_empty() {
        "archive".into()
    } else {
        host
    };
    let date = captured_at.unwrap_or_else(Utc::now).format("%Y-%m-%d");
    let mime = content_type.split(';').next().unwrap_or("").trim();
    let ext = match format {
        ArchiveFormat::Html => "html",
        ArchiveFormat::Pdf => "pdf",
        ArchiveFormat::File => match mime {
            "application/pdf" => "pdf",
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "audio/mpeg" => "mp3",
            "audio/ogg" | "video/ogg" => "ogg",
            "audio/wav" => "wav",
            "audio/flac" => "flac",
            "audio/mp4" => "m4a",
            "video/mp4" => "mp4",
            "video/webm" | "audio/webm" => "webm",
            "application/zip" => "zip",
            "text/plain" => "txt",
            "text/csv" => "csv",
            "application/json" => "json",
            _ => "bin",
        },
    };
    format!("{host}-{date}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day() -> Option<DateTime<Utc>> {
        Some(Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap())
    }

    #[test]
    fn html_copies_are_sandboxed_and_framed_only_here() {
        let h = copy_headers(
            ArchiveFormat::Html,
            "text/html",
            b"<p>",
            false,
            Some("example.com"),
            day(),
        );
        let csp = h[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
        assert!(csp.starts_with("sandbox "));
        assert!(!csp.contains("allow-scripts") && !csp.contains("allow-same-origin"));
        assert!(csp.contains("frame-ancestors 'self'"));
        assert_eq!(h[header::X_FRAME_OPTIONS], "SAMEORIGIN");
        assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(h[header::CACHE_CONTROL], "private, no-store");
        assert!(!h.contains_key(header::CONTENT_DISPOSITION));
    }

    #[test]
    fn active_files_only_download() {
        for (ct, body) in [
            ("image/svg+xml", b"<svg onload=alert(1)>".as_slice()),
            ("text/html", b"<script>".as_slice()),
            ("application/pdf", b"<html>".as_slice()),
        ] {
            let h = copy_headers(ArchiveFormat::File, ct, body, false, Some("x.org"), day());
            assert_eq!(h[header::CONTENT_TYPE], "application/octet-stream", "{ct}");
            assert!(h[header::CONTENT_DISPOSITION]
                .to_str()
                .unwrap()
                .starts_with("attachment;"));
        }
        let pdf = copy_headers(
            ArchiveFormat::File,
            "application/pdf",
            b"%PDF-1.7",
            false,
            None,
            day(),
        );
        assert_eq!(pdf[header::CONTENT_TYPE], "application/pdf");
        assert!(!pdf.contains_key(header::CONTENT_DISPOSITION));
    }

    #[test]
    fn downloads_are_named_after_the_site_and_day() {
        let h = copy_headers(
            ArchiveFormat::Pdf,
            "application/pdf",
            b"%PDF-1.7",
            true,
            Some("docs.rs"),
            day(),
        );
        assert_eq!(
            h[header::CONTENT_DISPOSITION],
            "attachment; filename=\"docs.rs-2026-11-03.pdf\""
        );
        assert_eq!(
            download_name(
                ArchiveFormat::File,
                "image/png",
                Some("a\"b\r\n.com"),
                day()
            ),
            "ab.com-2026-11-03.png"
        );
        assert_eq!(
            download_name(ArchiveFormat::Html, "", None, day()),
            "archive-2026-11-03.html"
        );
    }
}
