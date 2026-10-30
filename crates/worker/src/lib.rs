use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;
use linkdean_db::{self as db};
use linkdean_search::SearchClient;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct EnrichOpts {
    pub preview_dir: PathBuf,
    pub browser_ws_url: Option<String>,
}

pub async fn enrich_bookmark(
    pool: &PgPool,
    search: &SearchClient,
    user_id: Uuid,
    bookmark_id: Uuid,
    opts: &EnrichOpts,
) {
    let Ok(raw) = db::get_bookmark_raw(pool, user_id, bookmark_id).await else {
        return;
    };
    if raw.is_vault || raw.url.is_empty() {
        return;
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Linkdean/0.1 (+https://github.com/arsamsarabi/linkdean)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("http client: {e}");
            return;
        }
    };

    if let Ok(resp) = client.get(&raw.url).send().await {
        if resp.status().is_success() {
            if let Ok(body) = resp.text().await {
                let url = raw.url.clone();
                let parsed = tokio::task::spawn_blocking(move || parse_meta(&body))
                    .await
                    .unwrap_or_default();

                let title = if raw.title.is_empty() || raw.title == raw.url {
                    parsed.title.as_deref()
                } else {
                    None
                };
                let description = if raw.description.is_none() {
                    parsed.description.as_deref()
                } else {
                    None
                };
                let favicon = parsed.favicon.map(|href| absolutize(&url, &href));
                let preview = if raw.preview_image_url.is_none() {
                    parsed.preview_image.map(|href| absolutize(&url, &href))
                } else {
                    None
                };

                if let Err(e) = db::update_bookmark_metadata(
                    pool,
                    bookmark_id,
                    title,
                    description,
                    favicon.as_deref(),
                    preview.as_deref(),
                )
                .await
                {
                    warn!("metadata update: {e}");
                }
            }
        }
    }

    if !raw.has_preview {
        if let Some(browser_url) = opts.browser_ws_url.as_deref() {
            match capture_screenshot(browser_url, &raw.url).await {
                Ok(jpeg) => {
                    let path = preview_file(&opts.preview_dir, bookmark_id);
                    if let Some(parent) = path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    match tokio::fs::write(&path, &jpeg).await {
                        Ok(()) => {
                            if let Err(e) =
                                db::set_bookmark_has_preview(pool, bookmark_id, true).await
                            {
                                warn!("has_preview update: {e}");
                            } else {
                                info!(%bookmark_id, "saved preview screenshot");
                            }
                        }
                        Err(e) => warn!("preview write: {e}"),
                    }
                }
                Err(e) => warn!(%bookmark_id, "screenshot failed: {e}"),
            }
        }
    }

    if let Ok(updated) = db::get_bookmark_raw(pool, user_id, bookmark_id).await {
        if let Ok(Some(view)) = db::bookmark_to_view(pool, updated, None).await {
            let _ = search.upsert_bookmark(user_id, &view).await;
        }
    }
    info!(%bookmark_id, "enriched bookmark metadata");
}

pub fn preview_file(dir: &Path, bookmark_id: Uuid) -> PathBuf {
    dir.join(format!("{bookmark_id}.jpg"))
}

/// Rate-limited backfill for bookmarks missing local screenshots.
pub async fn backfill_previews(pool: PgPool, search: SearchClient, opts: EnrichOpts) {
    let Ok(rows) = db::list_bookmarks_needing_preview(&pool).await else {
        return;
    };
    if rows.is_empty() {
        return;
    }
    info!(count = rows.len(), "preview backfill starting");
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(2));
    let mut joins = Vec::new();
    for (user_id, bookmark_id) in rows {
        let permit = sem.clone().acquire_owned().await;
        let Ok(permit) = permit else {
            break;
        };
        let pool = pool.clone();
        let search = search.clone();
        let opts = opts.clone();
        joins.push(tokio::spawn(async move {
            enrich_bookmark(&pool, &search, user_id, bookmark_id, &opts).await;
            drop(permit);
        }));
    }
    for j in joins {
        let _ = j.await;
    }
    info!("preview backfill finished");
}

async fn capture_screenshot(browser_http: &str, page_url: &str) -> anyhow::Result<Vec<u8>> {
    let ws = resolve_browser_ws(browser_http).await?;
    let (mut browser, mut handler) = Browser::connect(ws).await?;
    let handle = tokio::spawn(async move {
        while let Some(_ev) = handler.next().await {}
    });

    let result = async {
        let page = browser.new_page("about:blank").await?;
        // Viewport via CDP (Page::set_viewport removed in chromiumoxide 0.7).
        page.execute(
            chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
                .width(1280)
                .height(720)
                .device_scale_factor(1.0)
                .mobile(false)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;

        tokio::time::timeout(Duration::from_secs(20), page.goto(page_url))
            .await
            .map_err(|_| anyhow::anyhow!("navigation timeout"))??;

        // Let late paint settle briefly.
        tokio::time::sleep(Duration::from_millis(800)).await;

        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Jpeg)
            .quality(80)
            .build();
        let jpeg = tokio::time::timeout(Duration::from_secs(10), page.screenshot(params))
            .await
            .map_err(|_| anyhow::anyhow!("screenshot timeout"))??;
        let _ = page.close().await;
        Ok::<Vec<u8>, anyhow::Error>(jpeg)
    }
    .await;

    let _ = browser.close().await;
    handle.abort();
    result
}

/// Fetch `/json/version` and rewrite 127.0.0.1/localhost to the HTTP host
/// (Chrome inside Docker often advertises loopback WS URLs).
async fn resolve_browser_ws(browser_http: &str) -> anyhow::Result<String> {
    let base = browser_http.trim_end_matches('/');
    if base.starts_with("ws://") || base.starts_with("wss://") {
        return Ok(base.to_string());
    }
    let version_url = format!("{base}/json/version");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let v: serde_json::Value = client.get(&version_url).send().await?.json().await?;
    let ws = v
        .get("webSocketDebuggerUrl")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow::anyhow!("no webSocketDebuggerUrl from {version_url}"))?;

    let http = url::Url::parse(base)?;
    let host = http
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("bad BROWSER_WS_URL host"))?;
    let port = http.port_or_known_default().unwrap_or(9222);

    let mut parsed = url::Url::parse(ws)?;
    let _ = parsed.set_host(Some(host));
    let _ = parsed.set_port(Some(port));
    Ok(parsed.to_string())
}

#[derive(Default)]
struct Meta {
    title: Option<String>,
    description: Option<String>,
    favicon: Option<String>,
    preview_image: Option<String>,
}

fn parse_meta(body: &str) -> Meta {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(body);
    let title_sel = Selector::parse("title").ok();
    let desc_sel = Selector::parse("meta[name=\"description\"]").ok();
    let og_desc_sel = Selector::parse("meta[property=\"og:description\"]").ok();
    let icon_sel = Selector::parse("link[rel~=\"icon\"]").ok();
    let og_image_sel = Selector::parse("meta[property=\"og:image\"]").ok();
    let og_image_secure_sel = Selector::parse("meta[property=\"og:image:secure_url\"]").ok();
    let twitter_image_sel = Selector::parse("meta[name=\"twitter:image\"]").ok();
    let twitter_image_prop_sel = Selector::parse("meta[property=\"twitter:image\"]").ok();

    let meta_content = |sel: Option<Selector>| -> Option<String> {
        sel.and_then(|s| doc.select(&s).next())
            .and_then(|el| el.value().attr("content"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let description = meta_content(desc_sel).or_else(|| meta_content(og_desc_sel));
    let preview_image = meta_content(og_image_sel)
        .or_else(|| meta_content(og_image_secure_sel))
        .or_else(|| meta_content(twitter_image_sel))
        .or_else(|| meta_content(twitter_image_prop_sel));

    Meta {
        title: title_sel
            .and_then(|s| doc.select(&s).next())
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty()),
        description,
        favicon: icon_sel
            .and_then(|s| doc.select(&s).next())
            .and_then(|el| el.value().attr("href"))
            .map(|s| s.to_string()),
        preview_image,
    }
}

fn absolutize(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") || href.starts_with("data:") {
        return href.to_string();
    }
    if let Ok(base_url) = url::Url::parse(base) {
        if let Ok(joined) = base_url.join(href) {
            return joined.to_string();
        }
    }
    href.to_string()
}
