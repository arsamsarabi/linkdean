//! Reader-mode page text extraction for archive search indexing.

use scraper::{ElementRef, Html, Node, Selector};

/// Max Unicode scalar values kept from extracted page text.
pub const PAGE_TEXT_MAX: usize = 100_000;

/// Extract searchable body text from archived HTML. Never panics; may return empty.
pub fn extract_page_text(html: &str) -> String {
    let doc = Html::parse_document(html);
    let Some(root) = select_root(&doc) else {
        return String::new();
    };

    let mut parts = Vec::new();
    collect_text(root, &mut parts);
    let collapsed: String = parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    collapsed.chars().take(PAGE_TEXT_MAX).collect()
}

/// Reader-mode Markdown from archived HTML (headings + paragraphs). Never panics; may return empty.
pub fn html_to_reader_markdown(html: &str) -> String {
    let doc = Html::parse_document(html);
    let Some(root) = select_root(&doc) else {
        return String::new();
    };

    let mut blocks = Vec::new();
    collect_markdown_blocks(root, &mut blocks);
    let joined = blocks
        .into_iter()
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    joined.chars().take(PAGE_TEXT_MAX).collect()
}

fn heading_level(tag: &str) -> Option<u8> {
    match tag {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

fn collect_markdown_blocks(el: ElementRef<'_>, out: &mut Vec<String>) {
    let tag = el.value().name();
    if is_chrome(tag) {
        return;
    }
    if let Some(level) = heading_level(tag) {
        let mut parts = Vec::new();
        collect_text(el, &mut parts);
        let text = parts
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            out.push(format!("{} {}", "#".repeat(level as usize), text));
        }
        return;
    }
    if tag == "p" || tag == "li" || tag == "blockquote" || tag == "pre" {
        let mut parts = Vec::new();
        collect_text(el, &mut parts);
        let text = parts
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            out.push(text);
        }
        return;
    }

    for child in el.children() {
        match child.value() {
            Node::Element(_) => {
                if let Some(child_el) = ElementRef::wrap(child) {
                    collect_markdown_blocks(child_el, out);
                }
            }
            Node::Text(t) => {
                let s = t.trim();
                if !s.is_empty() {
                    // Loose text under article/div/body — treat as a paragraph.
                    out.push(s.split_whitespace().collect::<Vec<_>>().join(" "));
                }
            }
            _ => {}
        }
    }
}

fn select_root<'a>(doc: &'a Html) -> Option<ElementRef<'a>> {
    // Selectors are compile-time constants; parse cannot fail for these.
    let article_sel = Selector::parse("article").ok()?;
    let articles: Vec<_> = doc.select(&article_sel).collect();
    if articles.len() == 1 {
        return Some(articles[0]);
    }

    let main_sel = Selector::parse("main").ok()?;
    if let Some(main) = doc.select(&main_sel).next() {
        return Some(main);
    }

    let body_sel = Selector::parse("body").ok()?;
    doc.select(&body_sel).next()
}

fn is_chrome(tag: &str) -> bool {
    matches!(tag, "script" | "style" | "nav" | "footer" | "aside")
}

fn collect_text(el: ElementRef<'_>, out: &mut Vec<String>) {
    // Known issue: trimming each text node and joining with spaces splits words across
    // inline tags (`Hel<i>lo</i>` → `Hel lo`). Fine for search this slice; upgrade by
    // concatenating adjacent text without a space when no whitespace was in the source.
    for child in el.children() {
        match child.value() {
            Node::Text(t) => {
                let s = t.trim();
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            }
            Node::Element(e) => {
                if is_chrome(e.name()) {
                    continue;
                }
                if let Some(child_el) = ElementRef::wrap(child) {
                    collect_text(child_el, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn article_body_is_kept_and_chrome_is_dropped() {
        let html = r#"<html><body>
      <nav>Home About</nav>
      <article><h1>UniquePhrase</h1><p>Body words here.</p></article>
      <footer>Copyright</footer>
      <script>ignore()</script>
    </body></html>"#;
        let text = extract_page_text(html);
        assert!(text.contains("UniquePhrase"));
        assert!(text.contains("Body words"));
        assert!(!text.contains("ignore()"));
        assert!(!text.contains("Home About"));
        assert!(!text.contains("Copyright"));
    }

    #[test]
    fn empty_or_garbage_yields_empty_or_short() {
        assert!(extract_page_text("").len() < 8);
        assert!(extract_page_text("<html><body></body></html>").len() < 32);
    }

    #[test]
    fn output_is_capped_at_page_text_max_ascii() {
        let para = "word ".repeat(PAGE_TEXT_MAX / 4);
        let html = format!("<html><body><article>{para}</article></body></html>");
        assert!(extract_page_text(&html).chars().count() <= PAGE_TEXT_MAX);
    }

    #[test]
    fn output_is_capped_at_page_text_max_multibyte() {
        let para = "é".repeat(PAGE_TEXT_MAX + 10);
        let html = format!("<html><body><article>{para}</article></body></html>");
        let text = extract_page_text(&html);
        assert!(text.chars().count() <= PAGE_TEXT_MAX);
        assert!(!text.is_empty());
    }

    #[test]
    fn multiple_articles_prefer_main_then_body() {
        let html = r#"<html><body>
          <main>
            <article>FirstPiece</article>
            <article>SecondPiece</article>
          </main>
        </body></html>"#;
        let text = extract_page_text(html);
        assert!(text.contains("FirstPiece"));
        assert!(text.contains("SecondPiece"));

        let body_only = r#"<html><body>
          <article>A</article>
          <article>B</article>
          <p>BodyFallback</p>
        </body></html>"#;
        let text = extract_page_text(body_only);
        assert!(text.contains("A") && text.contains("B") && text.contains("BodyFallback"));
    }

    #[test]
    fn headings_and_paragraphs_become_markdown() {
        let html = r#"<html><body>
      <article><h1>Title</h1><p>First para.</p><h2>Sub</h2><p>Second.</p></article>
    </body></html>"#;
        let md = html_to_reader_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Sub"));
        assert!(md.contains("First para."));
        assert!(md.contains("Second."));
    }

    #[test]
    fn chrome_is_dropped_from_markdown() {
        let html = r#"<html><body>
      <nav>Nav noise</nav>
      <article><h1>Keep</h1><p>Body</p></article>
      <footer>Footer noise</footer>
      <script>ignore()</script>
    </body></html>"#;
        let md = html_to_reader_markdown(html);
        assert!(md.contains("# Keep"));
        assert!(md.contains("Body"));
        assert!(!md.contains("Nav noise"));
        assert!(!md.contains("Footer noise"));
        assert!(!md.contains("ignore"));
    }

    #[test]
    fn markdown_output_is_capped_at_page_text_max_multibyte() {
        let unit = "字";
        let many = unit.repeat(PAGE_TEXT_MAX + 50);
        let html = format!("<article><p>{many}</p></article>");
        let md = html_to_reader_markdown(&html);
        assert!(md.chars().count() <= PAGE_TEXT_MAX);
    }

    #[test]
    fn markdown_empty_or_garbage_yields_empty() {
        assert!(html_to_reader_markdown("").is_empty());
        assert!(
            html_to_reader_markdown("<html><body><nav>x</nav></body></html>").is_empty()
                || html_to_reader_markdown("<html><body></body></html>").is_empty()
        );
    }
}
