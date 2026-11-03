//! Development-only pages. Mounted only when `ALLOW_DEFAULT_SECRETS=true`.

use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use linkdean_core::Error;
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dev/emails", get(index))
        .route("/dev/emails/{name}", get(preview))
}

async fn index() -> Html<String> {
    let items: String = linkdean_mail::template_names()
        .map(|n| {
            format!(
                r#"<li><a href="/api/dev/emails/{n}">{n}</a> · <a href="/api/dev/emails/{n}?format=text">text</a></li>"#
            )
        })
        .collect();
    Html(format!(
        "<!doctype html><meta charset=utf-8><title>Email templates</title>\
         <body style=\"font-family:system-ui;padding:2rem\"><h1>Email templates</h1>\
         <p>Each renders with its <code>.sample.json</code>. Edit a template, wait for the API to rebuild, reload.</p>\
         <ul>{items}</ul></body>"
    ))
}

#[derive(Deserialize)]
struct PreviewQuery {
    format: Option<String>,
}

async fn preview(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<PreviewQuery>,
) -> ApiResult<Response> {
    let data = linkdean_mail::sample(&name).ok_or(Error::NotFound)?;
    let site = linkdean_mail::Site::new(&state.cfg.app_url);
    let r = linkdean_mail::render(&name, &data, &site).// The sample is ours, so a render failure is a bug, not a bad request.
        map_err(|e| Error::msg(e.to_string()))?;
    Ok(if q.format.as_deref() == Some("text") {
        format!("Subject: {}\n\n{}", r.subject, r.text).into_response()
    } else {
        Html(r.html).into_response()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn index_lists_every_template() {
        let Html(page) = index().await;
        let mut names = linkdean_mail::template_names().peekable();
        assert!(names.peek().is_some());
        for name in names {
            assert!(
                page.contains(&format!(r#"href="/api/dev/emails/{name}""#)),
                "{name} missing from {page}"
            );
        }
    }
}
