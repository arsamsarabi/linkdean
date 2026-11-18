//! Sidebar version badge: is a newer Linkdean release out? Admins only, since only they can
//! upgrade. The server asks GitHub (people's browsers never do) and keeps the answer for hours.

use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use utoipa_axum::{router::OpenApiRouter, routes};

const RELEASES_URL: &str = "https://api.github.com/repos/arsamsarabi/linkdean/releases?per_page=30";
const FRESH_FOR: Duration = Duration::from_secs(6 * 60 * 60);
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(30 * 60);

/// The last answer and when it goes stale. One lock, so concurrent requests share one fetch.
static CACHE: Mutex<Option<(Instant, Option<Value>)>> = Mutex::const_new(None);

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(update_status))
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
}

/// `v1.2.3` or `1.2.3`. Anything else (such as `v1.2.3-rc1`) is not a release we offer.
fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.strip_prefix('v').unwrap_or(v).split('.');
    let out = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(out)
}

/// A release is breaking when its notes have a line starting "Breaking", such as a
/// `### Breaking` heading or `BREAKING: …`.
fn is_breaking(body: &str) -> bool {
    body.lines().any(|l| {
        l.trim_start_matches(|c: char| c == '#' || c.is_whitespace())
            .get(..8)
            .is_some_and(|w| w.eq_ignore_ascii_case("breaking"))
    })
}

/// The newest release above `current`, and how big the jump is. A breaking release anywhere in
/// between makes the whole jump `breaking`, so skipping over one still warns.
fn assess(current: &str, releases: &[Release]) -> Option<Value> {
    let cur = parse(current)?;
    let newer: Vec<(&Release, (u64, u64, u64))> = releases
        .iter()
        .filter(|r| !r.draft)
        .filter_map(|r| parse(&r.tag_name).map(|v| (r, v)))
        .filter(|(_, v)| *v > cur)
        .collect();
    let (latest, v) = newer.iter().max_by_key(|(_, v)| *v)?;
    let level = if newer
        .iter()
        .any(|(r, _)| is_breaking(r.body.as_deref().unwrap_or("")))
    {
        "breaking"
    } else if v.0 > cur.0 {
        "major"
    } else if v.1 > cur.1 {
        "minor"
    } else {
        "patch"
    };
    Some(json!({
        "latest": latest.tag_name.trim_start_matches('v'),
        "level": level,
        "url": latest.html_url,
    }))
}

async fn fetch_releases() -> anyhow::Result<Vec<Release>> {
    Ok(reqwest::Client::new()
        .get(RELEASES_URL)
        .header(
            "User-Agent",
            concat!("linkdean/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

#[utoipa::path(
    get,
    operation_id = "admin_update_status",
    path = "/admin/update",
    tag = "admin",
    responses(
        (status = 200, description = "`update` is null when this is the newest release, the check is off or GitHub could not be reached; otherwise `{ latest, level: patch|minor|major|breaking, url }`", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn update_status(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let current = env!("CARGO_PKG_VERSION");
    if !state.cfg.update_check {
        return Ok(Json(json!({ "current": current, "update": null })));
    }
    let mut cache = CACHE.lock().await;
    let update = match &*cache {
        Some((until, update)) if Instant::now() < *until => update.clone(),
        _ => {
            let (update, ttl) = match fetch_releases().await {
                Ok(releases) => (assess(current, &releases), FRESH_FOR),
                Err(e) => {
                    tracing::warn!(error = %e, "update check: GitHub releases fetch failed");
                    (None, RETRY_AFTER_FAILURE)
                }
            };
            *cache = Some((Instant::now() + ttl, update.clone()));
            update
        }
    };
    Ok(Json(json!({ "current": current, "update": update })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(tag: &str, body: &str) -> Release {
        Release {
            tag_name: tag.into(),
            body: Some(body.into()),
            html_url: format!("https://example.test/{tag}"),
            draft: false,
        }
    }

    fn level(current: &str, releases: &[Release]) -> Option<String> {
        assess(current, releases).map(|v| v["level"].as_str().unwrap().to_string())
    }

    #[test]
    fn up_to_date_or_older_releases_mean_no_update() {
        assert_eq!(
            level("0.0.12", &[rel("v0.0.12", ""), rel("v0.0.11", "")]),
            None
        );
        assert_eq!(level("0.0.12", &[]), None);
    }

    #[test]
    fn level_follows_the_biggest_version_part_that_moved() {
        assert_eq!(
            level("0.0.12", &[rel("v0.0.13", "")]).as_deref(),
            Some("patch")
        );
        assert_eq!(
            level("0.0.12", &[rel("v0.1.0", "")]).as_deref(),
            Some("minor")
        );
        assert_eq!(
            level("0.9.9", &[rel("v1.0.0", "")]).as_deref(),
            Some("major")
        );
    }

    #[test]
    fn a_breaking_release_anywhere_in_the_jump_wins() {
        let rs = [
            rel("v0.0.15", "## What's Changed"),
            rel("v0.0.14", "### Breaking\n* Drop the old export format"),
            rel("v0.0.13", ""),
        ];
        let v = assess("0.0.12", &rs).unwrap();
        assert_eq!(v["level"], "breaking");
        assert_eq!(v["latest"], "0.0.15");
        assert_eq!(v["url"], "https://example.test/v0.0.15");
    }

    #[test]
    fn breaking_in_an_already_installed_release_does_not_count() {
        let rs = [rel("v0.0.13", ""), rel("v0.0.12", "### Breaking")];
        assert_eq!(level("0.0.12", &rs).as_deref(), Some("patch"));
    }

    #[test]
    fn drafts_and_odd_tags_are_ignored() {
        let mut draft = rel("v0.0.20", "");
        draft.draft = true;
        let rs = [draft, rel("v0.0.19-rc1", ""), rel("nightly", "")];
        assert_eq!(level("0.0.12", &rs), None);
    }

    #[test]
    fn breaking_marker_needs_a_line_that_starts_with_it() {
        assert!(is_breaking("### Breaking changes"));
        assert!(is_breaking("BREAKING: sessions reset"));
        assert!(!is_breaking("* Fix a non-breaking bug"));
        assert!(!is_breaking(""));
    }
}
