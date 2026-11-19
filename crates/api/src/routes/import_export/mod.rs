use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::Json;
use linkdean_core::agent_skill::zip_stored;
use linkdean_core::import_export::{
    build_linkdean_export, export_netscape, parse_linkdean_json, parse_netscape, parse_pocket_csv,
    parse_raindrop_csv, DuplicateDecisions, ImportPreview, ParsedImport,
};
use linkdean_core::{crypto, ArchiveFormat, Error, ResolvedLimits, VaultKeyring};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(
            OpenApiRouter::new()
                .routes(routes!(import))
                .layer(DefaultBodyLimit::max(MAX_IMPORT_BODY)),
        )
        .merge(
            OpenApiRouter::new()
                .routes(routes!(preview))
                .layer(DefaultBodyLimit::max(MAX_IMPORT_BODY)),
        )
        .routes(routes!(export))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ImportBody {
    pub format: String,
    pub content: String,
    /// What to do with bookmarks already in the library. Missing: create them all, as before.
    #[serde(default)]
    pub duplicates: DuplicateDecisions,
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ExportQuery {
    #[serde(default = "default_format")]
    pub format: String,
    /// When set (and not `0`/`false`), the response is a zip with the export file plus archives.
    include_archives: Option<String>,
}

fn default_format() -> String {
    "json".into()
}

fn want_archives(flag: Option<&str>) -> bool {
    flag.is_some_and(|v| v != "0" && v != "false")
}

fn archive_file_ext(format: ArchiveFormat, content_type: Option<&str>) -> &'static str {
    match format {
        ArchiveFormat::Html => "html",
        ArchiveFormat::Pdf => "pdf",
        ArchiveFormat::File => match content_type.unwrap_or("") {
            t if t.starts_with("image/png") => "png",
            t if t.starts_with("image/jpeg") => "jpg",
            t if t.starts_with("image/gif") => "gif",
            t if t.starts_with("image/webp") => "webp",
            t if t.starts_with("audio/") => "audio",
            t if t.starts_with("video/") => "video",
            t if t.starts_with("application/pdf") => "pdf",
            _ => "bin",
        },
    }
}

/// JSON escaping can grow the file on the way in. The handler enforces the real limit.
const MAX_IMPORT_BODY: usize = 2 * crate::quotas::HARD_IMPORT_BYTES;

/// Users with an import in progress. Imports run inside the request, so this lives as long
/// as the handler does and a restart clears it.
static RUNNING: std::sync::Mutex<Option<std::collections::HashSet<uuid::Uuid>>> =
    std::sync::Mutex::new(None);

/// Holds the user's import slot until dropped, including when the handler errors.
struct ImportSlot(uuid::Uuid);

impl ImportSlot {
    /// One import at a time per user: a second one would race the first for the same
    /// quota and double the enrichment work.
    fn acquire(user_id: uuid::Uuid) -> Result<Self, Error> {
        let mut running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        if !running.get_or_insert_with(Default::default).insert(user_id) {
            return Err(Error::Conflict(
                "an import is already running; wait for it to finish".into(),
            ));
        }
        Ok(Self(user_id))
    }
}

impl Drop for ImportSlot {
    fn drop(&mut self) {
        let mut running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(set) = running.as_mut() {
            set.remove(&self.0);
        }
    }
}

/// Checks the size against the import limit, picks the parser for `format` and parses off
/// the async runtime. Returns the lower-cased format, the parsed file and the limits.
async fn parse_import(
    state: &AppState,
    format: &str,
    content: String,
) -> ApiResult<(String, ParsedImport, ResolvedLimits)> {
    let limits = crate::quotas::limits_for(&state.pool).await?;
    let max_bytes = crate::quotas::effective_import_max(&limits);
    if content.len() > max_bytes {
        return Err(Error::QuotaExceeded {
            meter: db::METER_IMPORT,
            limit: max_bytes as i64,
            used: content.len() as i64,
        }
        .into());
    }
    let format = format.to_ascii_lowercase();
    let parser = match format.as_str() {
        "netscape" | "html" => parse_netscape,
        "json" | "linkdean" => parse_linkdean_json,
        "pocket" => parse_pocket_csv,
        "raindrop" => parse_raindrop_csv,
        _ => {
            return Err(
                Error::BadRequest("format must be netscape|json|pocket|raindrop".into()).into(),
            )
        }
    };
    let parsed = tokio::task::spawn_blocking(move || parser(&content))
        .await
        .map_err(|e| Error::msg(format!("import parse: {e}")))??;
    Ok((format, parsed, limits))
}

/// What an import of this file would do: which bookmarks are already in the library, and
/// how many folders would be reused. Writes nothing.
#[utoipa::path(
    post,
    operation_id = "import_export_preview",
    path = "/import/preview",
    tag = "import",
    responses(
        (status = 200, description = "OK", body = ImportPreview),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn preview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<ImportBody>,
) -> ApiResult<Json<ImportPreview>> {
    let rate = crate::rate_limit::import_preview_limit(user.id);
    state.limiter.check(&rate)?;
    let _slot = ImportSlot::acquire(user.id)?;
    state.limiter.record(&rate);
    let (_, parsed, _) = parse_import(&state, &body.format, body.content).await?;
    Ok(Json(
        db::preview_import(&state.pool, user.id, &parsed).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "import_export_import",
    path = "/import",
    tag = "import",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn import(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<ImportBody>,
) -> ApiResult<Json<Value>> {
    let rate = crate::rate_limit::import_limit(user.id);
    state.limiter.check(&rate)?;
    let _slot = ImportSlot::acquire(user.id)?;
    state.limiter.record(&rate);
    let (format, parsed, limits) = parse_import(&state, &body.format, body.content).await?;
    let plan = db::plan_import(&state.pool, user.id, &parsed, &body.duplicates).await?;

    // Refuse the whole import when the bookmark gauge would overflow. Skipped and replaced
    // duplicates add nothing.
    db::refuse_bookmark_quota(&state.pool, user.id, &limits, plan.creates() as i64).await?;

    let job = db::create_import_job(&state.pool, user.id, &format).await?;
    let mut counts = db::ImportCounts::default();
    let applied =
        db::apply_parsed_import_counted(&state.pool, user.id, parsed, &plan, &mut counts).await;

    // Whatever landed is searchable, even when the import stopped part way.
    match db::list_bookmarks(
        &state.pool,
        user.id,
        None,
        false,
        &VaultKeyring::default(),
        false,
    )
    .await
    {
        Ok(items) => {
            if let Err(e) = crate::search_index::upsert_many(&state, &items).await {
                tracing::warn!(user_id = %user.id, error = %e, "meili reindex after import failed");
            }
        }
        Err(e) => {
            tracing::warn!(user_id = %user.id, error = %e, "list bookmarks to reindex failed")
        }
    }

    if let Err(e) = applied {
        tracing::warn!(user_id = %user.id, error = %e, imported = counts.imported, "import stopped part way");
        db::finish_import_job(
            &state.pool,
            job.id,
            counts.total,
            counts.imported,
            counts.failed,
            Some(&e.to_string()),
        )
        .await?;
        return Err(e.into());
    }
    let job = db::finish_import_job(
        &state.pool,
        job.id,
        counts.total,
        counts.imported,
        counts.failed,
        None,
    )
    .await?;
    // Imports can be large, so fetch metadata and previews after responding.
    tokio::spawn(linkdean_worker::enrich_user_bookmarks(
        state.pool.clone(),
        state.search.clone(),
        state.enrich_opts(),
        user.id,
    ));
    // One wake for the whole import; ARCHIVE_CONCURRENCY paces Chrome.
    if counts.any_archive_queued {
        state.archive_wake.notify_one();
    }

    Ok(Json(json!({
        "job_id": job.id,
        "total": counts.total,
        "imported": counts.imported,
        "failed": counts.failed,
        "skipped": counts.skipped,
        "replaced": counts.replaced,
        "merged": counts.merged,
        "status": job.status,
    })))
}

#[utoipa::path(
    get,
    operation_id = "import_export_export",
    path = "/export",
    tag = "export",
    params(ExportQuery),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn export(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    session: VaultSession,
    Query(q): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    // Vault items are in the file, as plaintext, only for vaults this session has unlocked.
    let keyring = state.vault_keyring(&session, user.id).await;
    let (mut cats, bms, dashboards) = db::export_data(&state.pool, user.id, &keyring).await?;
    let format = q.format.to_ascii_lowercase();
    let (meta_name, meta_bytes): (String, Vec<u8>) = match format.as_str() {
        "html" | "netscape" => {
            // Locked, the HTML file keeps leaving vault folders out, as it always has.
            if keyring.is_empty() {
                cats.retain(|c| !c.is_vault);
            }
            (
                "bookmarks.html".into(),
                export_netscape(&bms, &cats).into_bytes(),
            )
        }
        "json" | "linkdean" => {
            let export = build_linkdean_export(cats, bms, dashboards);
            let body =
                serde_json::to_string_pretty(&export).map_err(|e| Error::msg(e.to_string()))?;
            ("export.json".into(), body.into_bytes())
        }
        _ => return Err(Error::BadRequest("format must be json|html".into()).into()),
    };

    if !want_archives(q.include_archives.as_deref()) {
        let content_type = if meta_name.ends_with(".html") {
            "text/html; charset=utf-8"
        } else {
            "application/json; charset=utf-8"
        };
        return Ok(([(header::CONTENT_TYPE, content_type)], meta_bytes).into_response());
    }

    let mut files: Vec<(String, Vec<u8>)> = vec![(meta_name, meta_bytes)];
    let rows = db::done_archives_for_owner(&state.pool, user.id).await?;
    for row in rows {
        let Some(name) = row.file_name.as_deref() else {
            continue;
        };
        let format = match row.format() {
            Ok(f) => f,
            Err(_) => continue,
        };
        let stored = match tokio::fs::read(state.archive_dir(row.bookmark_id).join(name)).await {
            Ok(b) => b,
            Err(_) => continue,
        };
        let bytes = match (row.key_vault_id, &row.nonce) {
            (Some(vault_id), Some(nonce)) => {
                let Some(key) = keyring.get(vault_id) else {
                    // Locked vault: skip the copy, same as JSON omitting locked vault plaintext.
                    continue;
                };
                match crypto::decrypt_archive(
                    key,
                    vault_id,
                    row.bookmark_id,
                    format,
                    &stored,
                    nonce,
                ) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(
                            bookmark_id = %row.bookmark_id,
                            error = %e,
                            "export archive decrypt failed"
                        );
                        continue;
                    }
                }
            }
            _ => stored,
        };
        let ext = archive_file_ext(format, row.content_type.as_deref());
        files.push((
            format!("archives/{}/{}.{}", row.bookmark_id, format.as_str(), ext),
            bytes,
        ));
    }

    let zip = zip_stored(&files);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    if let Ok(v) = HeaderValue::from_str("attachment; filename=\"linkdean-export.zip\"") {
        headers.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok((headers, zip).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_import_at_a_time_per_user() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let slot = ImportSlot::acquire(a).unwrap();
        assert!(matches!(ImportSlot::acquire(a), Err(Error::Conflict(_))));
        let other = ImportSlot::acquire(b).expect("another user is not blocked");
        drop(slot);
        let again = ImportSlot::acquire(a).expect("slot is freed on drop");
        drop((other, again));
    }

    #[test]
    fn want_archives_reads_common_flags() {
        assert!(!want_archives(None));
        assert!(!want_archives(Some("0")));
        assert!(!want_archives(Some("false")));
        assert!(want_archives(Some("1")));
        assert!(want_archives(Some("true")));
    }

    #[test]
    fn archive_file_ext_covers_formats() {
        assert_eq!(archive_file_ext(ArchiveFormat::Html, None), "html");
        assert_eq!(archive_file_ext(ArchiveFormat::Pdf, None), "pdf");
        assert_eq!(
            archive_file_ext(ArchiveFormat::File, Some("image/png")),
            "png"
        );
        assert_eq!(
            archive_file_ext(ArchiveFormat::File, Some("weird/x")),
            "bin"
        );
    }
}
