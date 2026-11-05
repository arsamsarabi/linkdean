use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::import_export::{
    build_linkdean_export, export_netscape, parse_linkdean_json, parse_netscape, parse_pocket_csv,
    parse_raindrop_csv,
};
use linkdean_core::Error;
use linkdean_core::VaultKeyring;
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/import",
            post(import).layer(DefaultBodyLimit::max(MAX_IMPORT_BODY)),
        )
        .route("/export", get(export))
}

#[derive(Deserialize)]
pub struct ImportBody {
    pub format: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct ExportQuery {
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".into()
}

/// JSON escaping can grow the file on the way in. The handler enforces the real limit.
const MAX_IMPORT_BODY: usize = 2 * crate::quotas::HARD_IMPORT_BYTES;

async fn import(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<ImportBody>,
) -> ApiResult<Json<Value>> {
    let limits = crate::quotas::limits_for(&state.pool).await?;
    let max_bytes = crate::quotas::effective_import_max(&limits);
    if body.content.len() > max_bytes {
        return Err(Error::QuotaExceeded {
            meter: db::METER_IMPORT,
            limit: max_bytes as i64,
            used: body.content.len() as i64,
        }
        .into());
    }
    let format = body.format.to_ascii_lowercase();
    let content = body.content;
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

    // Refuse the whole import when the bookmark gauge would overflow.
    db::refuse_bookmark_quota(&state.pool, user.id, &limits, parsed.bookmarks.len() as i64).await?;

    let job = db::create_import_job(&state.pool, user.id, &format).await?;
    let mut counts = db::ImportCounts::default();
    let applied = db::apply_parsed_import_counted(&state.pool, user.id, parsed, &mut counts).await;

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
            if let Err(e) = state.search.upsert_bookmarks(&items).await {
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
    let (total, imported, failed) = (counts.total, counts.imported, counts.failed);

    // Imports can be large, so fetch metadata and previews after responding.
    tokio::spawn(linkdean_worker::enrich_user_bookmarks(
        state.pool.clone(),
        state.search.clone(),
        state.enrich_opts(),
        user.id,
    ));

    Ok(Json(json!({
        "job_id": job.id,
        "total": total,
        "imported": imported,
        "failed": failed,
        "status": job.status,
    })))
}

async fn export(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ExportQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let (cats, bms, dashboards) = db::export_data(&state.pool, user.id).await?;
    let format = q.format.to_ascii_lowercase();
    match format.as_str() {
        "html" | "netscape" => {
            let html = export_netscape(&bms, &cats);
            Ok((
                [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
                html,
            )
                .into_response())
        }
        "json" | "linkdean" => {
            let export = build_linkdean_export(cats, bms, dashboards);
            let body =
                serde_json::to_string_pretty(&export).map_err(|e| Error::msg(e.to_string()))?;
            Ok((
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/json; charset=utf-8",
                )],
                body,
            )
                .into_response())
        }
        _ => Err(Error::BadRequest("format must be json|html".into()).into()),
    }
}

use axum::response::IntoResponse;
