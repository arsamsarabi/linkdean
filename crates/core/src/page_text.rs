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
}
