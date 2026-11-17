//! Archived copies of pages: what a URL serves, a plain download for files, and a Chrome visit
//! for pages that keeps them as single-file HTML and/or PDF.

pub mod mhtml;

use crate::{guard_page_requests, is_challenge_page, net_guard, read_capped, resolve_browser_ws};
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureSnapshotFormat, CaptureSnapshotParams, PrintToPdfParams,
};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use futures::StreamExt;
use std::time::Duration;

const USER_AGENT: &str = "Linkdean/0.1 (+https://github.com/arsamsarabi/linkdean)";
const STEP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, PartialEq)]
pub enum CaptureError {
    /// Over the item cap.
    TooBig,
    /// The server answered 404 or 410.
    Gone(u16),
    /// A bot check stood in front of the page.
    Challenge,
    Failed(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooBig => write!(f, "too big"),
            Self::Gone(code) => write!(f, "gone (HTTP {code})"),
            Self::Challenge => write!(f, "bot challenge page"),
            Self::Failed(e) => write!(f, "{e}"),
        }
    }
}

fn failed(e: impl std::fmt::Display) -> CaptureError {
    CaptureError::Failed(e.to_string())
}

#[derive(Debug, PartialEq)]
pub enum Probe {
    Page,
    File { content_type: String },
    Gone(u16),
}

/// What the URL serves, from the response headers alone. Anything unclear (an error, 403, 429)
/// is left for Chrome to try, since sites often refuse plain clients but show pages to browsers.
pub async fn probe(url: &str) -> Probe {
    let Ok(client) = net_guard::public_client(Duration::from_secs(10), USER_AGENT) else {
        return Probe::Page;
    };
    let Ok(resp) = client.get(url).send().await else {
        return Probe::Page;
    };
    let status = resp.status().as_u16();
    if status == 404 || status == 410 {
        return Probe::Gone(status);
    }
    if !resp.status().is_success() {
        return Probe::Page;
    }
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    probe_type(&ct)
}

fn probe_type(content_type: &str) -> Probe {
    let mime = content_type.split(';').next().unwrap_or("").trim();
    if mime.is_empty() || mime == "text/html" || mime == "application/xhtml+xml" {
        Probe::Page
    } else {
        Probe::File {
            content_type: mime.to_string(),
        }
    }
}

/// Download a file of at most `max` bytes. Returns the bytes and the served type.
pub async fn fetch_file(url: &str, max: usize) -> Result<(Vec<u8>, String), CaptureError> {
    let client =
        net_guard::public_client(Duration::from_secs(120), USER_AGENT).map_err(failed)?;
    let resp = client.get(url).send().await.map_err(failed)?;
    let status = resp.status().as_u16();
    if status == 404 || status == 410 {
        return Err(CaptureError::Gone(status));
    }
    if !resp.status().is_success() {
        return Err(failed(format!("HTTP {status}")));
    }
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    if resp.content_length().is_some_and(|n| n > max as u64) {
        return Err(CaptureError::TooBig);
    }
    let bytes = read_capped(resp, max, false).await.map_err(|e| {
        if e.to_string().contains("larger than") || e.to_string().contains("too large") {
            CaptureError::TooBig
        } else {
            failed(e)
        }
    })?;
    Ok((bytes, ct))
}

/// The type a stored copy may be shown inline with, judged from its bytes. `None` means it is
/// only ever downloaded: HTML, SVG and anything else a browser might run.
pub fn inline_type(content_type: &str, bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    if let Some(image) = crate::sniff_image_ct(bytes).filter(|t| *t != "image/svg+xml") {
        return Some(image);
    }
    let mime = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    [
        "audio/mpeg",
        "audio/ogg",
        "audio/wav",
        "audio/webm",
        "audio/mp4",
        "audio/flac",
        "video/mp4",
        "video/webm",
        "video/ogg",
    ]
    .into_iter()
    .find(|t| *t == mime)
}

#[derive(Debug, Default)]
pub struct Captured {
    pub html: Option<Vec<u8>>,
    pub pdf: Option<Vec<u8>>,
    /// Where the page ended up after redirects.
    pub final_url: String,
}

/// Scrolls down at most ten screens so lazy images load, then back to the top.
const SCROLL_JS: &str = r#"(async () => {
  const pause = (ms) => new Promise((r) => setTimeout(r, ms));
  for (let i = 0; i < 10; i++) {
    const before = window.scrollY;
    window.scrollBy(0, window.innerHeight);
    await pause(300);
    if (window.scrollY === before) break;
  }
  window.scrollTo(0, 0);
  await pause(300);
  return true;
})()"#;

/// Bytes the page reports loading. Cross-origin resources without timing headers count as
/// zero, so this only catches pages that are plainly too big before Chrome serialises them.
const LOADED_BYTES_JS: &str = r#"performance.getEntriesByType('navigation')
  .concat(performance.getEntriesByType('resource'))
  .reduce((n, e) => n + (e.decodedBodySize || 0), 0)"#;

async fn eval(page: &chromiumoxide::Page, js: &str) -> Result<serde_json::Value, CaptureError> {
    let params = EvaluateParams::builder()
        .expression(js)
        .await_promise(true)
        .return_by_value(true)
        .build()
        .map_err(failed)?;
    let result = tokio::time::timeout(STEP_TIMEOUT, page.evaluate(params))
        .await
        .map_err(|_| failed("script timeout"))?
        .map_err(failed)?;
    Ok(result.value().cloned().unwrap_or_default())
}

/// One visit to `url` in the shared Chrome, behind the private-address guard.
pub async fn capture_page(
    browser_http: &str,
    url: &str,
    want_html: bool,
    want_pdf: bool,
    limits: &mhtml::Limits,
) -> Result<Captured, CaptureError> {
    let ws = resolve_browser_ws(browser_http).await.map_err(failed)?;
    // Chrome is shared, so never call `browser.close()`: it shuts the whole process down.
    let (browser, mut handler) = Browser::connect(ws).await.map_err(failed)?;
    let handle = tokio::spawn(async move { while let Some(_ev) = handler.next().await {} });
    let page = browser.new_page("about:blank").await;
    let result = async {
        let page = page.as_ref().map_err(failed)?;
        page.execute(
            chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
                .width(1280)
                .height(800)
                .device_scale_factor(1.0)
                .mobile(false)
                .build()
                .map_err(failed)?,
        )
        .await
        .map_err(failed)?;
        let guard = guard_page_requests(page).await.map_err(failed)?;
        let outcome = visit(page, url, want_html, want_pdf, limits).await;
        guard.abort();
        outcome
    }
    .await;
    if let Ok(page) = page {
        let _ = page.close().await;
    }
    drop(browser);
    handle.abort();
    result
}

async fn visit(
    page: &chromiumoxide::Page,
    url: &str,
    want_html: bool,
    want_pdf: bool,
    limits: &mhtml::Limits,
) -> Result<Captured, CaptureError> {
    tokio::time::timeout(Duration::from_secs(20), page.goto(url))
        .await
        .map_err(|_| failed("navigation timeout"))?
        .map_err(failed)?;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let title = page.get_title().await.map_err(failed)?.unwrap_or_default();
    let html = page.content().await.map_err(failed)?;
    if is_challenge_page(&title, &html) {
        return Err(CaptureError::Challenge);
    }
    eval(page, SCROLL_JS).await?;
    let loaded = eval(page, LOADED_BYTES_JS).await?.as_f64().unwrap_or(0.0);
    if loaded > limits.max_total as f64 * 2.0 {
        return Err(CaptureError::TooBig);
    }
    let final_url = page
        .url()
        .await
        .map_err(failed)?
        .unwrap_or_else(|| url.to_string());

    let mut out = Captured {
        final_url,
        ..Default::default()
    };
    if want_html {
        let snapshot = CaptureSnapshotParams {
            format: Some(CaptureSnapshotFormat::Mhtml),
        };
        let mhtml = tokio::time::timeout(STEP_TIMEOUT, page.execute(snapshot))
            .await
            .map_err(|_| failed("snapshot timeout"))?
            .map_err(failed)?
            .result
            .data;
        let limits = *limits;
        let converted = tokio::task::spawn_blocking(move || {
            mhtml::to_single_html(mhtml.as_bytes(), &limits)
        })
        .await
        .map_err(failed)?;
        out.html = Some(match converted {
            Ok(html) => html,
            Err(mhtml::ConvertError::TooBig(_)) => return Err(CaptureError::TooBig),
            Err(e) => return Err(failed(e)),
        });
    }
    if want_pdf {
        let params = PrintToPdfParams::builder().print_background(true).build();
        let pdf = tokio::time::timeout(STEP_TIMEOUT, page.pdf(params))
            .await
            .map_err(|_| failed("PDF timeout"))?
            .map_err(failed)?;
        if pdf.len() > limits.max_total {
            return Err(CaptureError::TooBig);
        }
        out.pdf = Some(pdf);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_are_told_from_pages_by_their_type() {
        assert_eq!(probe_type("text/html; charset=utf-8"), Probe::Page);
        assert_eq!(probe_type(""), Probe::Page);
        assert_eq!(
            probe_type("application/pdf"),
            Probe::File {
                content_type: "application/pdf".into()
            }
        );
    }

    #[test]
    fn only_inert_types_are_shown_inline() {
        assert_eq!(inline_type("application/pdf", b"%PDF-1.7"), Some("application/pdf"));
        assert_eq!(
            inline_type("application/octet-stream", b"\x89PNG\r\n\x1a\n"),
            Some("image/png")
        );
        assert_eq!(inline_type("image/svg+xml", b"<svg onload=x>"), None);
        assert_eq!(inline_type("text/html", b"<html>"), None);
        assert_eq!(inline_type("video/mp4", b"\0\0\0 ftyp"), Some("video/mp4"));
        // A PDF label on HTML bytes does not make it a PDF.
        assert_eq!(inline_type("application/pdf", b"<html>"), None);
    }

    /// Needs the dev stack's Chrome: `CHROME_URL` (default `http://127.0.0.1:9222`).
    #[tokio::test]
    #[ignore = "needs Chrome"]
    async fn captures_a_real_page() {
        let chrome =
            std::env::var("CHROME_URL").unwrap_or_else(|_| "http://127.0.0.1:9222".into());
        let limits = mhtml::Limits {
            max_resource: 5 * 1024 * 1024,
            max_total: 25 * 1024 * 1024,
        };
        let got = capture_page(&chrome, "https://example.com/", true, true, &limits)
            .await
            .expect("capture");
        let html = String::from_utf8(got.html.expect("html")).unwrap();
        assert!(html.contains("Example Domain"), "{html}");
        assert!(!html.contains("<script"));
        assert!(got.pdf.expect("pdf").starts_with(b"%PDF-"));
        assert!(got.final_url.starts_with("https://example.com"));
    }
}
