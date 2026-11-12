//! HTML `content` conversion for Greenhouse API fields.

use ego_tree::NodeRef;
use scraper::node::Node;
use scraper::{ElementRef, Html};

/// Decode HTML entities repeatedly (e.g. `&amp;lt;` → `<`).
pub fn decode_entities_capped(input: &str) -> String {
    let mut s = input.to_string();
    for _ in 0..8 {
        let decoded = html_escape::decode_html_entities(&s).to_string();
        if decoded == s {
            break;
        }
        s = decoded;
    }
    s
}

const SKIP_TAGS: &[&str] = &["script", "style", "noscript"];

/// Structure-preserving plain text from Greenhouse `content` HTML/entities (source order).
pub fn greenhouse_content_to_plain(content: &str) -> String {
    let decoded = decode_entities_capped(content);
    let frag = Html::parse_fragment(&decoded);
    let mut out: Vec<String> = Vec::new();
    let root = frag.tree.root();
    for child in root.children() {
        walk(child, &mut out);
    }
    if out.is_empty() {
        let plain: String = frag.root_element().text().collect::<Vec<_>>().join(" ");
        return collapse_ws(&plain);
    }
    collapse_ws(&out.join("\n"))
}

fn walk(node: NodeRef<'_, Node>, out: &mut Vec<String>) {
    match node.value() {
        Node::Text(t) => {
            let line = normalize_inline(&t);
            if !line.is_empty() {
                out.push(line);
            }
        }
        Node::Element(el) => {
            let tag = el.name().to_ascii_lowercase();
            if SKIP_TAGS.contains(&tag.as_str()) {
                return;
            }
            match tag.as_str() {
                "br" => out.push(String::new()),
                "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let Some(el) = ElementRef::wrap(node) else {
                        return;
                    };
                    let lines = inline_block_lines(el);
                    for line in lines {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if tag.starts_with('h') {
                            out.push(trimmed.to_string());
                        } else {
                            out.push(trimmed.to_string());
                        }
                    }
                }
                "li" => {
                    let Some(el) = ElementRef::wrap(node) else {
                        return;
                    };
                    let lines = li_lines(el);
                    for line in lines {
                        if !line.trim().is_empty() {
                            out.push(line);
                        }
                    }
                }
                "ul" | "ol" => {
                    for child in node.children() {
                        if child
                            .value()
                            .as_element()
                            .is_some_and(|e| e.name().eq_ignore_ascii_case("li"))
                        {
                            walk(child, out);
                        }
                    }
                }
                _ => {
                    for child in node.children() {
                        walk(child, out);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Text inside a block tag, preserving `<br>` as separate output lines (same block).
fn inline_block_lines(el: ElementRef<'_>) -> Vec<String> {
    let mut lines = vec![String::new()];
    fn recur<'a>(node: NodeRef<'a, Node>, lines: &mut Vec<String>) {
        match node.value() {
            Node::Text(t) => {
                if let Some(last) = lines.last_mut() {
                    append_inline_text(last, &t);
                }
            }
            Node::Element(el) => {
                let tag = el.name().to_ascii_lowercase();
                if SKIP_TAGS.contains(&tag.as_str()) {
                    return;
                }
                if tag == "br" {
                    lines.push(String::new());
                    return;
                }
                if matches!(
                    tag.as_str(),
                    "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li"
                ) {
                    return;
                }
                if let Some(first) = node.children().next() {
                    if let Node::Text(t) = first.value() {
                        if let Some(last) = lines.last_mut() {
                            if needs_space_before_inline(last, &t) {
                                last.push(' ');
                            }
                        }
                    }
                }
                for child in node.children() {
                    recur(child, lines);
                }
            }
            _ => {}
        }
    }
    for child in el.children() {
        recur(child, &mut lines);
    }
    lines
        .into_iter()
        .map(|l| normalize_inline(&l))
        .filter(|l| !l.is_empty())
        .collect()
}

fn li_lines(el: ElementRef<'_>) -> Vec<String> {
    let mut bullets = Vec::new();
    let mut direct = String::new();
    for child in el.children() {
        match child.value() {
            Node::Text(t) => append_inline_text(&mut direct, &t),
            Node::Element(inner) => {
                let tag = inner.name().to_ascii_lowercase();
                if SKIP_TAGS.contains(&tag.as_str()) {
                    continue;
                }
                if matches!(tag.as_str(), "ul" | "ol") {
                    if !direct.is_empty() {
                        bullets.push(format!("- {direct}"));
                        direct.clear();
                    }
                    for sub in child.children() {
                        if sub
                            .value()
                            .as_element()
                            .is_some_and(|e| e.name().eq_ignore_ascii_case("li"))
                        {
                            if let Some(sub_el) = ElementRef::wrap(sub) {
                                for line in li_lines(sub_el) {
                                    bullets.push(format!("  {line}"));
                                }
                            }
                        }
                    }
                } else if matches!(tag.as_str(), "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                    let Some(child_el) = ElementRef::wrap(child) else {
                        continue;
                    };
                    let parts = inline_block_lines(child_el);
                    if !direct.is_empty() {
                        bullets.push(format!("- {direct}"));
                        direct.clear();
                    }
                    for p in parts {
                        bullets.push(format!("- {p}"));
                    }
                } else {
                    let Some(child_el) = ElementRef::wrap(child) else {
                        continue;
                    };
                    if let Some(first) = child.children().next() {
                        if let Node::Text(t) = first.value() {
                            if needs_space_before_inline(&direct, &t) {
                                direct.push(' ');
                            }
                        }
                    }
                    let extra = inline_block_lines(child_el).join(" ");
                    if !extra.is_empty() {
                        if !direct.is_empty() {
                            direct.push(' ');
                        }
                        direct.push_str(&extra);
                    }
                }
            }
            _ => {}
        }
    }
    if !direct.is_empty() {
        bullets.push(format!("- {direct}"));
    }
    bullets
}

fn normalize_inline(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Join inline text nodes; respect explicit whitespace in HTML between inline tags.
fn append_inline_text(buf: &mut String, raw: &str) {
    if raw.is_empty() {
        return;
    }
    let leading_ws = raw.chars().next().is_some_and(|c| c.is_whitespace());
    let trailing_ws = raw.chars().last().is_some_and(|c| c.is_whitespace());
    let piece = normalize_inline(raw);
    if piece.is_empty() && !leading_ws && !trailing_ws {
        return;
    }
    if !buf.is_empty() && !buf.ends_with(' ') && leading_ws {
        buf.push(' ');
    }
    if !piece.is_empty() {
        buf.push_str(&piece);
    }
    if trailing_ws && !buf.ends_with(' ') {
        buf.push(' ');
    }
}

fn needs_space_before_inline(buf: &str, next_raw: &str) -> bool {
    if buf.is_empty() || buf.ends_with(' ') {
        return false;
    }
    if next_raw.chars().next().is_some_and(|c| c.is_whitespace()) {
        return false;
    }
    let piece = normalize_inline(next_raw);
    let a = buf.chars().last();
    let b = piece.chars().next();
    matches!(a, Some(c) if c.is_alphanumeric()) && matches!(b, Some(c) if c.is_alphanumeric())
}

fn collapse_ws(s: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in s.lines() {
        let t = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !t.is_empty() {
            lines.push(t);
        }
    }
    lines.join("\n")
}

const STOP: &[&str] = &[
    "the", "and", "or", "a", "an", "to", "of", "in", "for", "with", "our", "we", "you", "your",
];

/// Fixture-driven meaningful description check (no rigid char-only gate).
pub fn content_meaningful(plain: &str) -> bool {
    let t = plain.trim();
    if t.len() < 24 {
        return false;
    }
    let lower = t.to_lowercase();
    if lower.contains("equal opportunity") && t.len() < 120 {
        return false;
    }
    if lower.contains("gdpr") && lower.contains("privacy notice") && t.len() < 400 {
        return false;
    }
    let words = non_stop_words(lower.as_str());
    if lower.contains("equal opportunity employer") {
        let before = lower
            .split("equal opportunity employer")
            .next()
            .unwrap_or("")
            .trim();
        if non_stop_words(before).len() >= 8 {
            return true;
        }
        return false;
    }
    words.len() >= 8
}

fn non_stop_words(lower: &str) -> Vec<String> {
    lower
        .split_whitespace()
        .filter_map(|w| {
            let w = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if w.is_empty() || STOP.contains(&w.as_str()) {
                None
            } else {
                Some(w)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_html(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/greenhouse")
            .join(name);
        fs::read_to_string(&p).unwrap()
    }

    fn fixture_golden(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/greenhouse")
            .join(name);
        fs::read_to_string(&p).unwrap().trim_end().to_string()
    }

    fn assert_golden(html_file: &str, golden_file: &str) {
        let raw = fixture_html(html_file);
        let plain = greenhouse_content_to_plain(&raw);
        let expected = fixture_golden(golden_file);
        assert_eq!(
            plain, expected,
            "golden mismatch for {html_file}\n--- expected ---\n{expected}\n--- got ---\n{plain}"
        );
    }

    #[test]
    fn multiply_encoded_entities() {
        let raw = "Hello &amp;lt;strong&amp;gt;team&amp;lt;/strong&amp;gt;";
        let plain = greenhouse_content_to_plain(raw);
        assert!(plain.contains("team"));
    }

    #[test]
    fn short_valid_posting() {
        let text = "- Ship APIs\n- Lead design\n- Mentor juniors\nRust experience.";
        assert!(content_meaningful(text));
    }

    #[test]
    fn boilerplate_only_not_meaningful() {
        let text = "We are an equal opportunity employer.";
        assert!(!content_meaningful(text));
    }

    #[test]
    fn golden_order_paragraph_heading_list_paragraph() {
        assert_golden(
            "content-order-blocks.html",
            "content-order-blocks.golden.txt",
        );
    }

    #[test]
    fn golden_direct_container_mixed() {
        assert_golden(
            "content-direct-mixed.html",
            "content-direct-mixed.golden.txt",
        );
    }

    #[test]
    fn golden_nested_list_no_duplication() {
        assert_golden("content-nested-list.html", "content-nested-list.golden.txt");
    }

    #[test]
    fn golden_line_breaks() {
        assert_golden("content-line-breaks.html", "content-line-breaks.golden.txt");
    }

    #[test]
    fn golden_flexport_qualifications_in_order() {
        assert_golden(
            "content-flexport-qualifications.html",
            "content-flexport-qualifications.golden.txt",
        );
        let plain =
            greenhouse_content_to_plain(&fixture_html("content-flexport-qualifications.html"));
        let markers = [
            "About Flexport:",
            "The opportunity:",
            "You will:",
            "You should have:",
            "You will get:",
            "Commitment to Equal Opportunity",
        ];
        let mut last = 0;
        for m in markers {
            let pos = plain
                .find(m)
                .unwrap_or_else(|| panic!("missing qualification section {m}"));
            assert!(pos >= last, "out of order: {m}");
            last = pos;
        }
    }

    #[test]
    #[ignore = "regenerate golden .txt fixtures"]
    fn write_golden_text_fixtures() {
        let names = [
            "content-order-blocks",
            "content-direct-mixed",
            "content-nested-list",
            "content-line-breaks",
            "content-flexport-qualifications",
            "content-boilerplate-bullets-long",
            "content-inline-formatting",
            "content-meaningful-eoe-footer",
        ];
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/greenhouse");
        for name in names {
            let html = fs::read_to_string(base.join(format!("{name}.html"))).unwrap();
            let plain = greenhouse_content_to_plain(&html);
            fs::write(
                base.join(format!("{name}.golden.txt")),
                format!("{plain}\n"),
            )
            .unwrap();
        }
    }

    #[test]
    fn golden_inline_formatting_preserves_word_boundaries() {
        assert_golden(
            "content-inline-formatting.html",
            "content-inline-formatting.golden.txt",
        );
        let plain = greenhouse_content_to_plain(&fixture_html("content-inline-formatting.html"));
        assert_eq!(plain.lines().next().unwrap(), "Experience with Rust and Go");
        assert!(plain.contains("senior"));
        assert!(plain.contains("our board"));
        assert!(plain.contains("- Ship using Rust libraries daily"));
        assert!(plain.contains("Rusttooling"));
    }

    #[test]
    fn meaningful_job_with_eoe_footer() {
        let plain =
            greenhouse_content_to_plain(&fixture_html("content-meaningful-eoe-footer.html"));
        assert!(plain.contains("Responsibilities"));
        assert!(plain.contains("equal opportunity employer"));
        assert!(content_meaningful(&plain));
    }

    #[test]
    fn golden_boilerplate_bullets_not_meaningful() {
        let plain =
            greenhouse_content_to_plain(&fixture_html("content-boilerplate-bullets-long.html"));
        assert_golden(
            "content-boilerplate-bullets-long.html",
            "content-boilerplate-bullets-long.golden.txt",
        );
        assert!(!content_meaningful(&plain));
    }
}
