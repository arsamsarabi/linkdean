//! Turns Chrome's MHTML snapshot of a page into one self-contained HTML file: every image, style
//! and font the page loaded becomes a `data:` URI, and anything that could run is removed. The
//! copy is served under a sandbox CSP as well, so this is the first of two locks, not the only one.

use base64::Engine as _;
use lol_html::html_content::{ContentType, Element};
use lol_html::{element, rewrite_str, text, RewriteStrSettings};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// A resource larger than this is left out.
    pub max_resource: usize,
    /// The finished file may not be larger than this.
    pub max_total: usize,
}

#[derive(Debug, PartialEq)]
pub enum ConvertError {
    NotMhtml,
    NoDocument,
    TooBig(usize),
    Rewrite(String),
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotMhtml => write!(f, "the snapshot is not MHTML"),
            Self::NoDocument => write!(f, "the snapshot has no page in it"),
            Self::TooBig(n) => write!(f, "the page is {n} bytes, over the limit"),
            Self::Rewrite(e) => write!(f, "the page could not be rewritten: {e}"),
        }
    }
}

impl std::error::Error for ConvertError {}

struct Part {
    content_type: String,
    location: Option<String>,
    body: Vec<u8>,
}

/// Parts by absolute URL (`Content-Location`) and by `cid:` id.
struct Resources {
    parts: HashMap<String, Part>,
    max_resource: usize,
}

impl Resources {
    fn get(&self, base: Option<&Url>, raw: &str) -> Option<&Part> {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') || raw.starts_with("data:") {
            return None;
        }
        let mut url = match base {
            Some(b) => b.join(raw).ok()?,
            None => Url::parse(raw).ok()?,
        };
        url.set_fragment(None);
        self.parts.get(url.as_str()).or_else(|| self.parts.get(raw))
    }

    fn data_uri(&self, base: Option<&Url>, raw: &str) -> Option<String> {
        let part = self.get(base, raw)?;
        (part.body.len() <= self.max_resource).then(|| data_uri(&part.content_type, &part.body))
    }
}

fn data_uri(content_type: &str, body: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        clean_mime(content_type),
        base64::engine::general_purpose::STANDARD.encode(body)
    )
}

/// The bare type of a `Content-Type` value, or `application/octet-stream` when it looks odd.
fn clean_mime(ct: &str) -> String {
    let mime = ct
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let ok = mime.contains('/')
        && mime
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/.+-".contains(&b));
    if ok {
        mime
    } else {
        "application/octet-stream".into()
    }
}

pub fn to_single_html(mhtml: &[u8], limits: &Limits) -> Result<Vec<u8>, ConvertError> {
    // Inlined resources are about as large as the base64 parts they come from.
    if mhtml.len() / 4 > limits.max_total {
        return Err(ConvertError::TooBig(mhtml.len()));
    }
    let (headers, body) = split_head(mhtml).ok_or(ConvertError::NotMhtml)?;
    let headers = parse_headers(headers);
    let ct = header(&headers, "content-type").ok_or(ConvertError::NotMhtml)?;
    if !ct.to_ascii_lowercase().starts_with("multipart/related") {
        return Err(ConvertError::NotMhtml);
    }
    let boundary = param(ct, "boundary").ok_or(ConvertError::NotMhtml)?;
    let snapshot_location = header(&headers, "snapshot-content-location").map(str::to_string);

    let mut document: Option<Part> = None;
    let mut parts = HashMap::new();
    for raw in split_parts(body, &boundary) {
        let Some(part) = parse_part(raw) else {
            continue;
        };
        if document.is_none()
            && part
                .content_type
                .to_ascii_lowercase()
                .starts_with("text/html")
        {
            document = Some(part);
            continue;
        }
        if let Some(loc) = &part.location {
            let key = Url::parse(loc)
                .map(|mut u| {
                    u.set_fragment(None);
                    u.to_string()
                })
                .unwrap_or_else(|_| loc.clone());
            parts.insert(key, part);
        }
    }
    let document = document.ok_or(ConvertError::NoDocument)?;
    let base = document
        .location
        .as_deref()
        .or(snapshot_location.as_deref())
        .and_then(|u| Url::parse(u).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https"));
    let res = Resources {
        parts,
        max_resource: limits.max_resource,
    };
    let html = String::from_utf8_lossy(&document.body);
    let out = rewrite(&html, base.as_ref(), &res).map_err(ConvertError::Rewrite)?;
    if out.len() > limits.max_total {
        return Err(ConvertError::TooBig(out.len()));
    }
    Ok(out.into_bytes())
}

/// Elements that run code, pull in other documents, or move the page's base.
const REMOVED: &str = "script, noscript, iframe, frame, frameset, object, embed, applet, base, \
                       portal, meta[http-equiv]";

fn rewrite(html: &str, base: Option<&Url>, res: &Resources) -> Result<String, String> {
    let mut style_text = String::new();
    let settings = RewriteStrSettings::new()
        .append_element_content_handler(element!(REMOVED, |el| {
            el.remove();
            Ok(())
        }))
        .append_element_content_handler(element!("*", |el| {
            strip_active_attributes(el);
            if let Some(style) = el.get_attribute("style") {
                el.set_attribute("style", &inline_css(&style, base, res, 0))?;
            }
            Ok(())
        }))
        .append_element_content_handler(element!("head", |el| {
            el.prepend(
                r#"<meta charset="utf-8"><base target="_blank">"#,
                ContentType::Html,
            );
            Ok(())
        }))
        .append_element_content_handler(element!("link", |el| {
            let rel = el
                .get_attribute("rel")
                .unwrap_or_default()
                .to_ascii_lowercase();
            let href = el.get_attribute("href").unwrap_or_default();
            let rels: Vec<&str> = rel.split_ascii_whitespace().collect();
            if rels.contains(&"stylesheet") {
                match res.get(base, &href) {
                    Some(part) if part.body.len() <= res.max_resource => {
                        let own = part.location.as_deref().and_then(|u| Url::parse(u).ok());
                        let css = inline_css(
                            &String::from_utf8_lossy(&part.body),
                            own.as_ref().or(base),
                            res,
                            0,
                        );
                        let media = el
                            .get_attribute("media")
                            .map(|m| format!(" media=\"{}\"", html_attr(&m)))
                            .unwrap_or_default();
                        el.replace(
                            &format!("<style{media}>{}</style>", escape_style(&css)),
                            ContentType::Html,
                        );
                    }
                    _ => el.remove(),
                }
            } else if rels.contains(&"icon") {
                match res.data_uri(base, &href) {
                    Some(uri) => el.set_attribute("href", &uri)?,
                    None => el.remove(),
                }
            } else {
                el.remove();
            }
            Ok(())
        }))
        .append_element_content_handler(text!("style", |t| {
            style_text.push_str(t.as_str());
            if t.last_in_text_node() {
                let css = escape_style(&inline_css(&style_text, base, res, 0));
                t.replace(&css, ContentType::Html);
                style_text.clear();
            } else {
                t.remove();
            }
            Ok(())
        }))
        .append_element_content_handler(element!("img", |el| {
            let from_set = el
                .get_attribute("srcset")
                .and_then(|s| first_candidate(&s, base, res));
            let from_src = el.get_attribute("src").and_then(|s| res.data_uri(base, &s));
            if let Some(uri) = from_set.or(from_src) {
                el.set_attribute("src", &uri)?;
            }
            el.remove_attribute("srcset");
            el.remove_attribute("sizes");
            Ok(())
        }))
        .append_element_content_handler(element!("source", |el| {
            if let Some(set) = el.get_attribute("srcset") {
                match first_candidate(&set, base, res) {
                    Some(uri) => {
                        el.set_attribute("srcset", &uri)?;
                        el.remove_attribute("sizes");
                    }
                    // The <img> beside it shows the picture Chrome loaded.
                    None => el.remove(),
                }
            } else if let Some(uri) = el.get_attribute("src").and_then(|s| res.data_uri(base, &s)) {
                el.set_attribute("src", &uri)?;
            }
            Ok(())
        }))
        .append_element_content_handler(element!("video[poster]", |el| {
            if let Some(uri) = el
                .get_attribute("poster")
                .and_then(|s| res.data_uri(base, &s))
            {
                el.set_attribute("poster", &uri)?;
            }
            Ok(())
        }))
        .append_element_content_handler(element!("input[src]", |el| {
            if let Some(uri) = el.get_attribute("src").and_then(|s| res.data_uri(base, &s)) {
                el.set_attribute("src", &uri)?;
            }
            Ok(())
        }))
        .append_element_content_handler(element!("a[href], area[href]", |el| {
            // Gone when it was a script URL.
            let Some(href) = el.get_attribute("href") else {
                return Ok(());
            };
            if !href.trim_start().starts_with('#') {
                if let Some(abs) = base.and_then(|b| b.join(href.trim()).ok()) {
                    el.set_attribute("href", abs.as_str())?;
                }
            }
            Ok(())
        }));
    rewrite_str(html, settings).map_err(|e| e.to_string())
}

fn strip_active_attributes(el: &mut Element) {
    let names: Vec<(String, String)> = el
        .attributes()
        .iter()
        .map(|a| (a.name(), a.value()))
        .collect();
    for (name, value) in names {
        let script_url = matches!(
            name.as_str(),
            "href" | "src" | "action" | "formaction" | "xlink:href" | "poster" | "data"
        ) && is_script_url(&value);
        if name.starts_with("on") || script_url {
            el.remove_attribute(&name);
        }
    }
}

fn is_script_url(value: &str) -> bool {
    let squeezed: String = value
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .take(16)
        .collect::<String>()
        .to_ascii_lowercase();
    squeezed.starts_with("javascript:") || squeezed.starts_with("vbscript:")
}

/// The first `srcset` candidate Chrome loaded, as a data URI.
fn first_candidate(srcset: &str, base: Option<&Url>, res: &Resources) -> Option<String> {
    srcset
        .split(',')
        .filter_map(|c| c.split_ascii_whitespace().next())
        .find_map(|u| res.data_uri(base, u))
}

fn html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

/// Keeps inlined CSS from closing its `<style>` element early.
fn escape_style(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = find_ci(rest, "</style") {
        out.push_str(&rest[..i]);
        out.push_str("<\\/style");
        rest = &rest[i + "</style".len()..];
    }
    out.push_str(rest);
    out
}

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    hay.as_bytes()
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Inlines `@import`ed sheets (three levels deep) and every `url()` Chrome loaded.
fn inline_css(css: &str, base: Option<&Url>, res: &Resources, depth: u8) -> String {
    let css = if depth < 3 {
        inline_imports(css, base, res, depth)
    } else {
        css.to_string()
    };
    let mut out = String::with_capacity(css.len());
    let mut rest = css.as_str();
    while let Some(i) = find_ci(rest, "url(") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 4..];
        let Some((raw, used)) = css_url_arg(after) else {
            out.push_str(&rest[i..i + 4]);
            rest = after;
            continue;
        };
        match res.data_uri(base, raw) {
            Some(uri) => out.push_str(&format!("url(\"{uri}\")")),
            None => out.push_str(&rest[i..i + 4 + used]),
        }
        rest = &after[used..];
    }
    out.push_str(rest);
    out
}

/// The argument of a `url(` already consumed, and how many bytes up to and including `)`.
fn css_url_arg(s: &str) -> Option<(&str, usize)> {
    let trimmed = s.trim_start();
    let lead = s.len() - trimmed.len();
    if let Some(q) = trimmed.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let end = trimmed[1..].find(q)? + 1;
        let close = trimmed[end + 1..].find(')')? + end + 1;
        Some((&trimmed[1..end], lead + close + 1))
    } else {
        let close = trimmed.find(')')?;
        Some((trimmed[..close].trim(), lead + close + 1))
    }
}

fn inline_imports(css: &str, base: Option<&Url>, res: &Resources, depth: u8) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = find_ci(rest, "@import") {
        out.push_str(&rest[..i]);
        let after = &rest[i + "@import".len()..];
        let Some(end) = after.find(';') else {
            out.push_str(&rest[i..]);
            return out;
        };
        let rule = after[..end].trim();
        let (target, media) =
            if let Some(r) = rule.strip_prefix("url(").or(rule.strip_prefix("URL(")) {
                match css_url_arg(r) {
                    Some((u, used)) => (u, r[used..].trim()),
                    None => ("", ""),
                }
            } else if let Some(q) = rule.chars().next().filter(|c| *c == '"' || *c == '\'') {
                match rule[1..].find(q) {
                    Some(e) => (&rule[1..e + 1], rule[e + 2..].trim()),
                    None => ("", ""),
                }
            } else {
                ("", "")
            };
        match res.get(base, target) {
            Some(part) if part.body.len() <= res.max_resource => {
                let own = part.location.as_deref().and_then(|u| Url::parse(u).ok());
                let sheet = inline_css(
                    &String::from_utf8_lossy(&part.body),
                    own.as_ref().or(base),
                    res,
                    depth + 1,
                );
                if media.is_empty() {
                    out.push_str(&sheet);
                } else {
                    out.push_str(&format!("@media {media} {{{sheet}}}"));
                }
            }
            // Not loaded: drop it, since the copy can never fetch it.
            _ => {}
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn split_head(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let crlf = find_bytes(bytes, b"\r\n\r\n").map(|i| (i, 4));
    let lf = find_bytes(bytes, b"\n\n").map(|i| (i, 2));
    let (i, n) = match (crlf, lf) {
        (Some(a), Some(b)) => {
            if a.0 <= b.0 {
                a
            } else {
                b
            }
        }
        (a, b) => a.or(b)?,
    };
    Some((&bytes[..i], &bytes[i + n..]))
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Header lines, unfolded, as (lower-case name, value).
fn parse_headers(raw: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(raw);
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t']) {
            if let Some(last) = out.last_mut() {
                last.1.push(' ');
                last.1.push_str(line.trim());
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            out.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    out
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

fn param(value: &str, name: &str) -> Option<String> {
    value.split(';').skip(1).find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k.trim().eq_ignore_ascii_case(name)).then(|| v.trim().trim_matches('"').to_string())
    })
}

fn split_parts<'a>(body: &'a [u8], boundary: &str) -> Vec<&'a [u8]> {
    let delim = format!("--{boundary}");
    let delim = delim.as_bytes();
    let mut out = Vec::new();
    let mut rest = body;
    let Some(first) = find_bytes(rest, delim) else {
        return out;
    };
    rest = &rest[first + delim.len()..];
    loop {
        if rest.starts_with(b"--") {
            break;
        }
        let end = find_bytes(rest, delim).unwrap_or(rest.len());
        let mut part = &rest[..end];
        part = part
            .strip_prefix(b"\r\n")
            .or(part.strip_prefix(b"\n"))
            .unwrap_or(part);
        part = part
            .strip_suffix(b"\r\n")
            .or(part.strip_suffix(b"\n"))
            .unwrap_or(part);
        out.push(part);
        if end == rest.len() {
            break;
        }
        rest = &rest[end + delim.len()..];
    }
    out
}

fn parse_part(raw: &[u8]) -> Option<Part> {
    let (head, body) = split_head(raw)?;
    let headers = parse_headers(head);
    let content_type = header(&headers, "content-type")
        .unwrap_or("application/octet-stream")
        .to_string();
    let location = header(&headers, "content-location")
        .map(str::to_string)
        .or_else(|| {
            header(&headers, "content-id")
                .map(|id| format!("cid:{}", id.trim_matches(|c| c == '<' || c == '>')))
        });
    let encoding = header(&headers, "content-transfer-encoding")
        .unwrap_or("")
        .to_ascii_lowercase();
    let body = match encoding.as_str() {
        "base64" => {
            let clean: Vec<u8> = body
                .iter()
                .copied()
                .filter(|b| !b.is_ascii_whitespace())
                .collect();
            base64::engine::general_purpose::STANDARD
                .decode(clean)
                .ok()?
        }
        "quoted-printable" => decode_qp(body),
        _ => body.to_vec(),
    };
    Some(Part {
        content_type,
        location,
        body,
    })
}

fn decode_qp(input: &[u8]) -> Vec<u8> {
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'=' {
            if input[i + 1..].starts_with(b"\r\n") {
                i += 3;
                continue;
            }
            if input[i + 1..].starts_with(b"\n") {
                i += 2;
                continue;
            }
            if let (Some(h), Some(l)) = (
                input.get(i + 1).copied().and_then(hex),
                input.get(i + 2).copied().and_then(hex),
            ) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(input[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits {
        max_resource: 1_000,
        max_total: 100_000,
    };
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

    /// An MHTML file like Chrome's: the page as quoted-printable, other parts as base64.
    fn mhtml(page: &str, parts: &[(&str, &str, &[u8])]) -> Vec<u8> {
        let b = "----MultipartBoundary--test----";
        let mut out = format!(
            "From: <Saved by Blink>\r\nSnapshot-Content-Location: https://site.example/a/page\r\n\
             MIME-Version: 1.0\r\nContent-Type: multipart/related;\r\n\ttype=\"text/html\";\r\n\
             \tboundary=\"{b}\"\r\n\r\n\r\n--{b}\r\nContent-Type: text/html\r\n\
             Content-Transfer-Encoding: quoted-printable\r\n\
             Content-Location: https://site.example/a/page\r\n\r\n{page}\r\n"
        );
        for (loc, ct, body) in parts {
            let b64 = base64::engine::general_purpose::STANDARD.encode(body);
            out.push_str(&format!(
                "--{b}\r\nContent-Type: {ct}\r\nContent-Transfer-Encoding: base64\r\n\
                 Content-Location: {loc}\r\n\r\n{b64}\r\n"
            ));
        }
        out.push_str(&format!("--{b}--\r\n"));
        out.into_bytes()
    }

    fn convert(page: &str, parts: &[(&str, &str, &[u8])]) -> String {
        String::from_utf8(to_single_html(&mhtml(page, parts), &LIMITS).unwrap()).unwrap()
    }

    #[test]
    fn images_become_data_uris() {
        let out = convert(
            "<html><head></head><body><img src=3D\"pic.png\" srcset=3D\"big.png 2x\"></body></html>",
            &[("https://site.example/a/pic.png", "image/png", PNG)],
        );
        assert!(
            out.contains(r#"src="data:image/png;base64,iVBORw0KGgpmYWtl""#),
            "{out}"
        );
        assert!(!out.contains("srcset"));
        assert!(out.contains(r#"<head><meta charset="utf-8"><base target="_blank">"#));
    }

    #[test]
    fn stylesheets_and_their_urls_are_inlined_relative_to_the_sheet() {
        let css = b"@import 'more.css' print;\nbody{background:url(img/bg.png)}";
        let out = convert(
            "<html><head><link rel=3D\"stylesheet\" href=3D\"/s/main.css\"></head><body></body></html>",
            &[
                ("https://site.example/s/main.css", "text/css", css),
                ("https://site.example/s/more.css", "text/css", b"p{color:red}"),
                ("https://site.example/s/img/bg.png", "image/png", PNG),
            ],
        );
        assert!(!out.contains("<link"), "{out}");
        assert!(out.contains("<style>@media print {p{color:red}}"), "{out}");
        assert!(out.contains("url(\"data:image/png;base64,"), "{out}");
    }

    #[test]
    fn scripts_handlers_and_frames_are_removed() {
        let out = convert(
            "<html><head><script>alert(1)</script><base href=3D\"https://evil.example/\">\
             <meta http-equiv=3D\"refresh\" content=3D\"0\"></head><body onload=3D\"x()\">\
             <a href=3D\"javascript:alert(1)\" onclick=3D\"y()\">x</a><iframe src=3D\"f\"></iframe>\
             <object data=3D\"o\"></object><noscript>n</noscript>\
             <style>p{}</StYle><script>bad()</script></body></html>",
            &[],
        );
        for gone in [
            "<script",
            "alert",
            "onload",
            "onclick",
            "<iframe",
            "<object",
            "<noscript",
            "evil",
            "refresh",
            "bad()",
        ] {
            assert!(!out.contains(gone), "{gone} left in {out}");
        }
        assert!(out.contains("<a>x</a>"), "{out}");
    }

    #[test]
    fn style_text_cannot_close_its_element() {
        assert_eq!(escape_style("a</STYLE><b>"), "a<\\/style><b>");
    }

    #[test]
    fn oversized_resources_keep_their_url() {
        let big = vec![0u8; 2_000];
        let out = convert(
            "<html><head></head><body><img src=3D\"big.png\"></body></html>",
            &[("https://site.example/a/big.png", "image/png", &big)],
        );
        assert!(out.contains(r#"src="big.png""#), "{out}");
    }

    #[test]
    fn quoted_printable_decodes() {
        assert_eq!(decode_qp(b"a=3Db=\r\nc=C3=A9"), "a=bcé".as_bytes());
    }

    #[test]
    fn links_point_at_the_original_site() {
        let out = convert(
            "<html><head></head><body><a href=3D\"../b?q=3D1\">b</a><a href=3D\"#top\">t</a></body></html>",
            &[],
        );
        assert!(
            out.contains(r#"href="https://site.example/b?q=1""#),
            "{out}"
        );
        assert!(out.contains(r##"href="#top""##), "{out}");
    }

    #[test]
    fn size_and_shape_are_checked() {
        let tight = Limits {
            max_resource: 1_000,
            max_total: 50,
        };
        let page = "<html><head></head><body>".to_string() + &"x".repeat(200) + "</body></html>";
        assert!(matches!(
            to_single_html(&mhtml(&page, &[]), &tight),
            Err(ConvertError::TooBig(_))
        ));
        assert_eq!(
            to_single_html(b"<html>not mhtml</html>", &LIMITS),
            Err(ConvertError::NotMhtml)
        );
    }
}
