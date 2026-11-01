use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::import_export::{
    build_linkdean_export, export_netscape, parse_linkdean_json, parse_netscape, parse_pocket_csv,
    parse_raindrop_csv,
};
use linkdean_core::Error;
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/import", post(import))
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

const MAX_IMPORT_BYTES: usize = 10 * 1024 * 1024;

async fn import(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<ImportBody>,
) -> ApiResult<Json<Value>> {
    if body.content.len() > MAX_IMPORT_BYTES {
        return Err(Error::BadRequest("import too large (max 10 MiB)".into()).into());
    }
    let format = body.format.to_ascii_lowercase();
    let parsed = match format.as_str() {
        "netscape" | "html" => parse_netscape(&body.content)?,
        "json" | "linkdean" => parse_linkdean_json(&body.content)?,
        "pocket" => parse_pocket_csv(&body.content)?,
        "raindrop" => parse_raindrop_csv(&body.content)?,
        _ => {
            return Err(
                Error::BadRequest("format must be netscape|json|pocket|raindrop".into()).into(),
            )
        }
    };

    let job = db::create_import_job(&state.pool, user.id, &format).await?;
    let (total, imported, failed) = db::apply_parsed_import(&state.pool, user.id, parsed).await?;
    let job = db::finish_import_job(&state.pool, job.id, total, imported, failed, None).await?;

    // reindex non-vault
    if let Ok(items) = db::list_bookmarks(&state.pool, user.id, None, false, None, false).await {
        for v in items {
            if let Err(e) = state.search.upsert_bookmark(&v).await {
                tracing::warn!(bookmark_id = %v.id, error = %e, "meili reindex failed");
            }
        }
    }

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
    let (cats, bms) = db::export_data(&state.pool, user.id).await?;
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
            let export = build_linkdean_export(cats, bms);
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
