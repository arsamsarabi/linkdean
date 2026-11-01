use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;
use linkdean_core::LinkCheckStatus;
use linkdean_db::{self as db};
use linkdean_search::SearchClient;
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
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

                let favicon_remote = parsed
                    .favicon
                    .map(|href| absolutize(&url, &href))
                    .filter(|u| is_fetchable(u))
                    .or_else(|| raw.favicon_url.clone().filter(|u| is_http_url(u)));
                let favicon = resolve_cached_asset(
                    &client,
                    &opts.preview_dir,
                    bookmark_id,
                    AssetKind::Favicon,
                    favicon_remote,
                    raw.favicon_url.as_deref(),
                )
                .await;

                let preview_remote = parsed
                    .preview_image
                    .map(|href| absolutize(&url, &href))
                    .filter(|u| is_fetchable(u))
                    .or_else(|| raw.preview_image_url.clone().filter(|u| is_http_url(u)));
                let preview = resolve_cached_asset(
                    &client,
                    &opts.preview_dir,
                    bookmark_id,
                    AssetKind::Og,
                    preview_remote,
                    raw.preview_image_url.as_deref(),
                )
                .await;

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
        let owner = updated.owner_user_id;
        if let Ok(Some(view)) = db::bookmark_to_view(pool, updated, None, owner).await {
            let _ = search.upsert_bookmark(&view).await;
        }
    }
    info!(%bookmark_id, "enriched bookmark metadata");
}

pub fn preview_file(dir: &Path, bookmark_id: Uuid) -> PathBuf {
    dir.join(format!("{bookmark_id}.jpg"))
}

pub fn favicon_file(dir: &Path, bookmark_id: Uuid) -> PathBuf {
    dir.join(format!("{bookmark_id}.favicon"))
}

pub fn favicon_ct_file(dir: &Path, bookmark_id: Uuid) -> PathBuf {
    dir.join(format!("{bookmark_id}.favicon.ct"))
}

pub fn og_file(dir: &Path, bookmark_id: Uuid) -> PathBuf {
    dir.join(format!("{bookmark_id}.og"))
}

pub fn og_ct_file(dir: &Path, bookmark_id: Uuid) -> PathBuf {
    dir.join(format!("{bookmark_id}.og.ct"))
}

pub fn cached_favicon_url(bookmark_id: Uuid) -> String {
    format!("/api/bookmarks/{bookmark_id}/favicon")
}

pub fn cached_og_url(bookmark_id: Uuid) -> String {
    format!("/api/bookmarks/{bookmark_id}/og")
}

/// Remove screenshot + favicon + OG cache files (best-effort).
pub async fn purge_preview_assets(dir: &Path, bookmark_id: Uuid) {
    for path in [
        preview_file(dir, bookmark_id),
        favicon_file(dir, bookmark_id),
        favicon_ct_file(dir, bookmark_id),
        og_file(dir, bookmark_id),
        og_ct_file(dir, bookmark_id),
    ] {
        if let Err(e) = tokio::fs::remove_file(&path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!(?path, error = %e, "preview asset delete failed");
            }
        }
    }
}

#[derive(Clone, Copy)]
enum AssetKind {
    Favicon,
    Og,
}

impl AssetKind {
    fn max_bytes(self) -> usize {
        match self {
            Self::Favicon => 512 * 1024,
            Self::Og => 5 * 1024 * 1024,
        }
    }

    fn path(self, dir: &Path, id: Uuid) -> PathBuf {
        match self {
            Self::Favicon => favicon_file(dir, id),
            Self::Og => og_file(dir, id),
        }
    }

    fn ct_path(self, dir: &Path, id: Uuid) -> PathBuf {
        match self {
            Self::Favicon => favicon_ct_file(dir, id),
            Self::Og => og_ct_file(dir, id),
        }
    }

    fn local_url(self, id: Uuid) -> String {
        match self {
            Self::Favicon => cached_favicon_url(id),
            Self::Og => cached_og_url(id),
        }
    }

    fn default_ct(self) -> &'static str {
        match self {
            Self::Favicon => "image/png",
            Self::Og => "image/jpeg",
        }
    }
}

/// Cache remote image on disk; return local API URL (or remote fallback / None = no DB change).
async fn resolve_cached_asset(
    client: &reqwest::Client,
    dir: &Path,
    bookmark_id: Uuid,
    kind: AssetKind,
    remote: Option<String>,
    existing: Option<&str>,
) -> Option<String> {
    let local = kind.local_url(bookmark_id);
    let path = kind.path(dir, bookmark_id);
    let ct_path = kind.ct_path(dir, bookmark_id);

    if path.is_file() {
        if existing == Some(local.as_str()) {
            return None;
        }
        return Some(local);
    }

    let fetch_url = remote
        .filter(|u| is_fetchable(u))
        .or_else(|| existing.filter(|u| is_http_url(u)).map(|s| s.to_string()))?;

    match fetch_and_store(client, &fetch_url, &path, &ct_path, kind).await {
        Ok(()) => Some(local),
        Err(e) => {
            warn!(%bookmark_id, error = %e, "asset cache failed");
            if existing == Some(fetch_url.as_str()) {
                None
            } else if is_http_url(&fetch_url) {
                Some(fetch_url)
            } else {
                None
            }
        }
    }
}

async fn fetch_and_store(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    ct_path: &Path,
    kind: AssetKind,
) -> anyhow::Result<()> {
    let (bytes, ct) = if let Some(data) = parse_data_url(url) {
        data
    } else {
        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("HTTP {}", resp.status());
        }
        let header_ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
            .filter(|s| !s.is_empty());
        let bytes = resp.bytes().await?;
        if bytes.len() > kind.max_bytes() {
            anyhow::bail!("asset too large ({} bytes)", bytes.len());
        }
        let ct = header_ct
            .or_else(|| sniff_image_ct(&bytes).map(str::to_string))
            .unwrap_or_else(|| kind.default_ct().to_string());
        (bytes.to_vec(), ct)
    };

    if bytes.len() > kind.max_bytes() {
        anyhow::bail!("asset too large ({} bytes)", bytes.len());
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, &bytes).await?;
    let _ = tokio::fs::write(ct_path, ct.as_bytes()).await;
    Ok(())
}

fn parse_data_url(url: &str) -> Option<(Vec<u8>, String)> {
    let rest = url.strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    let ct = meta
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .trim()
        .to_string();
    let bytes = if meta.contains(";base64") {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(data.trim())
            .ok()?
    } else {
        // Percent-decoded plain data URLs are rare for images; skip if not base64.
        return None;
    };
    Some((bytes, ct))
}

fn sniff_image_ct(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']) {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if bytes.len() >= 4
        && bytes[0] == 0x00
        && bytes[1] == 0x00
        && bytes[2] == 0x01
        && bytes[3] == 0x00
    {
        return Some("image/x-icon");
    }
    let head = std::str::from_utf8(&bytes[..bytes.len().min(256)]).ok()?;
    let trimmed = head.trim_start();
    if trimmed.starts_with("<svg") || trimmed.contains("<svg") {
        return Some("image/svg+xml");
    }
    None
}

fn is_http_url(u: &str) -> bool {
    u.starts_with("http://") || u.starts_with("https://")
}

fn is_fetchable(u: &str) -> bool {
    is_http_url(u) || u.starts_with("data:image/")
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

#[derive(Clone, Debug)]
pub struct LinkCheckOpts {
    pub enabled: bool,
    pub tick_secs: u64,
    pub batch_size: i64,
    pub concurrency: usize,
    pub per_host_delay_ms: u64,
    pub recheck_days: i64,
}

impl Default for LinkCheckOpts {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_secs: 60,
            batch_size: 40,
            concurrency: 2,
            per_host_delay_ms: 1_000,
            recheck_days: 7,
        }
    }
}

#[derive(Debug)]
struct ProbeResult {
    status: LinkCheckStatus,
    status_code: Option<u16>,
    error: Option<String>,
}

fn classify_http(code: u16) -> LinkCheckStatus {
    match code {
        200..=399 | 401 | 403 => LinkCheckStatus::Ok,
        404 | 410 => LinkCheckStatus::Broken,
        429 | 500..=599 => LinkCheckStatus::Unverifiable,
        400..=499 => LinkCheckStatus::Broken,
        _ => LinkCheckStatus::Unverifiable,
    }
}

fn head_needs_get_fallback(code: u16) -> bool {
    matches!(code, 403 | 404 | 405 | 501)
}

fn is_blocked_host(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    matches!(
        h.as_str(),
        "localhost" | "localhost.localdomain" | "metadata.google.internal" | "metadata" | "0.0.0.0"
    ) || h.ends_with(".localhost")
        || h.ends_with(".local")
        || h.parse::<std::net::IpAddr>()
            .map(is_blocked_ip)
            .unwrap_or(false)
}

fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.is_unspecified()
        }
    }
}

async fn probe_url(client: &reqwest::Client, url: &str) -> ProbeResult {
    let Ok(parsed) = url::Url::parse(url) else {
        return ProbeResult {
            status: LinkCheckStatus::Broken,
            status_code: None,
            error: Some("invalid url".into()),
        };
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return ProbeResult {
            status: LinkCheckStatus::Unverifiable,
            status_code: None,
            error: Some("non-http scheme".into()),
        };
    }
    let Some(host) = parsed.host_str() else {
        return ProbeResult {
            status: LinkCheckStatus::Broken,
            status_code: None,
            error: Some("missing host".into()),
        };
    };
    if is_blocked_host(host) {
        return ProbeResult {
            status: LinkCheckStatus::Unverifiable,
            status_code: None,
            error: Some("blocked host".into()),
        };
    }

    match client.head(url).send().await {
        Ok(resp) => {
            let code = resp.status().as_u16();
            // 403 on HEAD is common while GET works — fall through via head_needs_get_fallback.
            if resp.status().is_success() || (300..400).contains(&code) || code == 401 {
                return ProbeResult {
                    status: classify_http(code),
                    status_code: Some(code),
                    error: None,
                };
            }
            if !head_needs_get_fallback(code) {
                return ProbeResult {
                    status: classify_http(code),
                    status_code: Some(code),
                    error: Some(format!("HTTP {code}")),
                };
            }
        }
        Err(_) => {
            // Many hosts reject HEAD outright — try GET.
        }
    }

    match client.get(url).send().await {
        Ok(resp) => {
            let code = resp.status().as_u16();
            // Drop body early; we only care about reachability.
            drop(resp);
            ProbeResult {
                status: classify_http(code),
                status_code: Some(code),
                error: if classify_http(code).is_problem() {
                    Some(format!("HTTP {code}"))
                } else {
                    None
                },
            }
        }
        Err(e) => ProbeResult {
            status: LinkCheckStatus::Unreachable,
            status_code: None,
            error: Some(truncate_err(&e.to_string())),
        },
    }
}

fn truncate_err(s: &str) -> String {
    const MAX: usize = 200;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!("{}…", &s[..MAX])
    }
}

async fn wait_per_host(
    last_by_host: &Mutex<HashMap<String, Instant>>,
    host: &str,
    delay: Duration,
) {
    loop {
        let sleep_for = {
            let mut map = last_by_host.lock().await;
            if let Some(prev) = map.get(host) {
                let elapsed = prev.elapsed();
                if elapsed < delay {
                    Some(delay - elapsed)
                } else {
                    map.insert(host.to_string(), Instant::now());
                    None
                }
            } else {
                map.insert(host.to_string(), Instant::now());
                None
            }
        };
        match sleep_for {
            Some(d) => tokio::time::sleep(d).await,
            None => break,
        }
    }
}

pub async fn check_bookmark_link(pool: &PgPool, bookmark_id: Uuid, url: &str) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::limited(8))
        .user_agent("LinkdeanLinkCheck/0.1 (+https://github.com/arsamsarabi/linkdean)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("link-check http client: {e}");
            return;
        }
    };

    let result = probe_url(&client, url).await;
    if let Err(e) = db::record_link_check(
        pool,
        bookmark_id,
        result.status,
        result.status_code.map(i32::from),
        result.error.as_deref(),
    )
    .await
    {
        warn!(%bookmark_id, error = %e, "record link check failed");
    }
}

/// Background loop: claim due bookmarks and probe URLs with global + per-host pacing.
pub async fn run_link_checker(pool: PgPool, opts: LinkCheckOpts) {
    if !opts.enabled {
        info!("link checker disabled");
        return;
    }
    info!(
        tick_secs = opts.tick_secs,
        concurrency = opts.concurrency,
        recheck_days = opts.recheck_days,
        "link checker starting"
    );
    let last_by_host = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));
    let recheck = chrono::Duration::days(opts.recheck_days.max(1));
    // Soft failures retry sooner so two-strike confirmation does not wait a full week.
    let soft_recheck = chrono::Duration::days(2).min(recheck);
    let per_host = Duration::from_millis(opts.per_host_delay_ms.max(100));
    let concurrency = opts.concurrency.max(1);

    loop {
        match db::claim_bookmarks_for_link_check(
            &pool,
            opts.batch_size.max(1),
            recheck,
            soft_recheck,
        )
        .await
        {
            Ok(batch) if !batch.is_empty() => {
                info!(count = batch.len(), "link-check batch");
                let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
                let mut joins = Vec::with_capacity(batch.len());
                for (owner_id, bookmark_id, url) in batch {
                    let _ = owner_id;
                    let host = url::Url::parse(&url)
                        .ok()
                        .and_then(|u| u.host_str().map(str::to_string))
                        .unwrap_or_else(|| "_".into());
                    let permit = match sem.clone().acquire_owned().await {
                        Ok(p) => p,
                        Err(_) => break,
                    };
                    let pool = pool.clone();
                    let last_by_host = last_by_host.clone();
                    joins.push(tokio::spawn(async move {
                        wait_per_host(&last_by_host, &host, per_host).await;
                        check_bookmark_link(&pool, bookmark_id, &url).await;
                        drop(permit);
                    }));
                }
                for j in joins {
                    let _ = j.await;
                }
            }
            Ok(_) => {}
            Err(e) => warn!(error = %e, "link-check claim failed"),
        }
        tokio::time::sleep(Duration::from_secs(opts.tick_secs.max(5))).await;
    }
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
    // Chrome is shared, so never call `browser.close()`: it shuts the whole process down.
    let (browser, mut handler) = Browser::connect(ws).await?;
    let handle = tokio::spawn(async move { while let Some(_ev) = handler.next().await {} });

    let page = browser.new_page("about:blank").await;
    let result = async {
        let page = page.as_ref().map_err(|e| anyhow::anyhow!("{e}"))?;
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
        Ok::<Vec<u8>, anyhow::Error>(jpeg)
    }
    .await;

    if let Ok(page) = page {
        let _ = page.close().await;
    }
    drop(browser);
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
    let http = with_ip_host(url::Url::parse(base)?).await?;
    let host = http
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("bad BROWSER_WS_URL host"))?;
    let port = http.port_or_known_default().unwrap_or(9222);

    let version_url = http.join("json/version")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let v: serde_json::Value = client
        .get(version_url.clone())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let ws = v
        .get("webSocketDebuggerUrl")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow::anyhow!("no webSocketDebuggerUrl from {version_url}"))?;

    let mut parsed = url::Url::parse(ws)?;
    let _ = parsed.set_host(Some(host));
    let _ = parsed.set_port(Some(port));
    Ok(parsed.to_string())
}

/// Chrome's DevTools server rejects a `Host` header that is a hostname other than
/// localhost, so a Compose service name has to be replaced by its IP address.
async fn with_ip_host(mut url: url::Url) -> anyhow::Result<url::Url> {
    let Some(url::Host::Domain(host)) = url.host() else {
        return Ok(url);
    };
    let port = url.port_or_known_default().unwrap_or(9222);
    let ip = tokio::net::lookup_host((host, port))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no address for BROWSER_WS_URL host"))?
        .ip();
    url.set_ip_host(ip)
        .map_err(|_| anyhow::anyhow!("bad BROWSER_WS_URL host"))?;
    Ok(url)
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

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn with_ip_host_replaces_hostnames_only() {
        let named = url::Url::parse("http://localhost:9222").unwrap();
        let resolved = super::with_ip_host(named).await.unwrap();
        assert!(matches!(
            resolved.host(),
            Some(url::Host::Ipv4(_) | url::Host::Ipv6(_))
        ));
        assert_eq!(resolved.port(), Some(9222));

        let ip = url::Url::parse("http://10.0.0.5:9222").unwrap();
        assert_eq!(super::with_ip_host(ip.clone()).await.unwrap(), ip);
    }

    use super::*;

    #[test]
    fn sniff_png_jpeg_ico() {
        assert_eq!(
            sniff_image_ct(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']),
            Some("image/png")
        );
        assert_eq!(
            sniff_image_ct(&[0xff, 0xd8, 0xff, 0xe0]),
            Some("image/jpeg")
        );
        assert_eq!(
            sniff_image_ct(&[0x00, 0x00, 0x01, 0x00, 0x01]),
            Some("image/x-icon")
        );
    }

    #[test]
    fn local_asset_urls() {
        let id = Uuid::nil();
        assert_eq!(
            cached_favicon_url(id),
            "/api/bookmarks/00000000-0000-0000-0000-000000000000/favicon"
        );
        assert_eq!(
            cached_og_url(id),
            "/api/bookmarks/00000000-0000-0000-0000-000000000000/og"
        );
    }

    #[test]
    fn classify_http_status() {
        assert_eq!(classify_http(200), LinkCheckStatus::Ok);
        assert_eq!(classify_http(301), LinkCheckStatus::Ok);
        assert_eq!(classify_http(401), LinkCheckStatus::Ok);
        assert_eq!(classify_http(403), LinkCheckStatus::Ok);
        assert_eq!(classify_http(404), LinkCheckStatus::Broken);
        assert_eq!(classify_http(410), LinkCheckStatus::Broken);
        assert_eq!(classify_http(429), LinkCheckStatus::Unverifiable);
        assert_eq!(classify_http(503), LinkCheckStatus::Unverifiable);
    }

    #[test]
    fn blocks_private_hosts() {
        assert!(is_blocked_host("localhost"));
        assert!(is_blocked_host("127.0.0.1"));
        assert!(is_blocked_host("10.0.0.5"));
        assert!(is_blocked_host("192.168.1.1"));
        assert!(is_blocked_host("169.254.169.254"));
        assert!(!is_blocked_host("example.com"));
    }
}
