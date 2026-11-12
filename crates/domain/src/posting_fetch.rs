//! URL validation, HTML retrieval constraints, and JobPosting extraction helpers.
//! All retrieved web content is untrusted; callers must not execute scripts.

use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

/// Extraction pipeline version (bump when heuristics change).
pub const POSTING_EXTRACTOR_VERSION: &str = "2";

/// Max plain-text from HTML fallback (separate from raw snapshot storage).
pub const MAX_FALLBACK_DESCRIPTION_CHARS: usize = 24_000;

pub const OUTCOME_RECOGNIZED: &str = "recognized";
pub const OUTCOME_PARTIAL: &str = "partial";
pub const OUTCOME_SOURCE_UNAVAILABLE: &str = "source_unavailable";
pub const OUTCOME_EXTRACTION_FAILED: &str = "extraction_failed";

pub const DIAG_OUTCOME_PREFIX: &str = "extraction_outcome:";

/// Hard cap on stored body; exceeding returns error (no silent truncate).
pub const MAX_POSTING_BODY_BYTES: usize = 2_000_000;

pub const DEFAULT_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

pub const MAX_POSTING_REDIRECTS: usize = 15;

#[derive(Debug, Error)]
pub enum PostingFetchError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("URL scheme must be http or https")]
    UnsupportedScheme,
    #[error("SSRF protection: host `{0}` is not allowed")]
    DisallowedHost(String),
    #[error("too many redirects (max {0})")]
    TooManyRedirects(usize),
    #[error("unsupported content-type: {0}")]
    UnsupportedContentType(String),
    #[error("response body exceeds maximum of {0} bytes")]
    BodyTooLarge(usize),
    #[error("network or HTTP error: {0}")]
    Transport(String),
    #[error("not found or empty response")]
    EmptyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedPostingPayload {
    pub title: Option<String>,
    pub employer: Option<String>,
    pub location: Option<String>,
    pub employment_type: Option<String>,
    pub work_mode: Option<String>,
    pub compensation: Option<crate::Compensation>,
    /// Plain-text description for display and future analysis (not raw HTML).
    pub description_plain: Option<String>,
}

impl Default for FetchedPostingPayload {
    fn default() -> Self {
        Self {
            title: None,
            employer: None,
            location: None,
            employment_type: None,
            work_mode: None,
            compensation: None,
            description_plain: None,
        }
    }
}

/// Result of parsing HTML (no network).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractPostingResult {
    pub payload: FetchedPostingPayload,
    pub diagnostics: Vec<String>,
}

/// Parse URL allowed for fetch (structure only).
pub fn validate_public_fetch_url(input: &str) -> Result<Url, PostingFetchError> {
    let u = Url::parse(input.trim()).map_err(|e| PostingFetchError::InvalidUrl(e.to_string()))?;
    match u.scheme() {
        "https" | "http" => {}
        _ => return Err(PostingFetchError::UnsupportedScheme),
    }
    let host = u
        .host_str()
        .ok_or_else(|| PostingFetchError::InvalidUrl("missing host".into()))?;
    assert_host_safe(host)?;
    Ok(u)
}

fn assert_host_safe(host: &str) -> Result<(), PostingFetchError> {
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost" {
        return Err(PostingFetchError::DisallowedHost(host.to_string()));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_disallowed_destination_ip(ip) {
            return Err(PostingFetchError::DisallowedHost(host.to_string()));
        }
        return Ok(());
    }
    Ok(())
}

/// Returns true when the address must not be contacted for posting ingestion.
pub fn is_disallowed_destination_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 127
                || o[0] == 0
                || (o[0] == 10)
                || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                || (o[0] == 192 && o[1] == 168)
                || (o[0] == 169 && o[1] == 254)
                || o[0] >= 224 // multicast + reserved
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local
        }
    }
}

/// Resolve host to a public socket address; connect only to returned addr (no second DNS at connect).
pub fn resolve_public_socket_addr(host: &str, port: u16) -> Result<SocketAddr, PostingFetchError> {
    assert_host_safe(host)?;
    let target = (host, port);
    let mut last_err = None;
    let mut allowed = None;
    for addr in target
        .to_socket_addrs()
        .map_err(|e| PostingFetchError::Transport(format!("DNS resolution failed: {e}")))?
    {
        if is_disallowed_destination_ip(addr.ip()) {
            last_err = Some(PostingFetchError::DisallowedHost(format!(
                "{} resolves to {}",
                host,
                addr.ip()
            )));
            continue;
        }
        allowed = Some(addr);
        break;
    }
    allowed.ok_or_else(|| {
        last_err.unwrap_or_else(|| {
            PostingFetchError::Transport(format!("no public addresses for host `{host}`"))
        })
    })
}

/// After redirects, re-check the final host (DNS was resolved by client; we check name).
pub fn validate_final_url_after_redirects(final_url: &str) -> Result<(), PostingFetchError> {
    let u = Url::parse(final_url).map_err(|e| PostingFetchError::InvalidUrl(e.to_string()))?;
    let host = u
        .host_str()
        .ok_or_else(|| PostingFetchError::InvalidUrl("missing host".into()))?;
    assert_host_safe(host)
}

/// SHA-256 hex digest of body (for integrity).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Parse `extraction_outcome:*` from snapshot/interpretation diagnostics.
pub fn parse_extraction_outcome(diagnostics: &[String]) -> Option<String> {
    diagnostics
        .iter()
        .find_map(|d| d.strip_prefix(DIAG_OUTCOME_PREFIX).map(|s| s.to_string()))
}

/// Short list label when title is missing.
pub fn outcome_list_label(outcome: &str) -> &'static str {
    match outcome {
        OUTCOME_SOURCE_UNAVAILABLE => "Unavailable at source",
        OUTCOME_EXTRACTION_FAILED => "Extraction failed",
        OUTCOME_PARTIAL => "Partial extraction",
        OUTCOME_RECOGNIZED => "Posting",
        _ => "Saved posting",
    }
}

/// Extract JobPosting from JSON-LD scripts in HTML; merge first reasonable candidate.
pub fn extract_from_html(html: &str) -> ExtractPostingResult {
    let mut diagnostics = Vec::new();
    let mut payload = FetchedPostingPayload::default();

    let unavailable = detect_source_unavailable(html);
    let mut had_json_ld = false;
    let mut used_fallback = false;

    if unavailable {
        diagnostics.push("source page signals job unavailable".into());
    } else {
        let mut job_posting_count = 0usize;
        for block in extract_json_ld_blocks(html) {
            match serde_json::from_str::<Value>(&block) {
                Err(e) => diagnostics.push(format!("json-ld parse skip: {e}")),
                Ok(v) => {
                    if let Some(jp) = find_job_posting_value(&v) {
                        job_posting_count += 1;
                        if job_posting_count == 1 {
                            merge_job_posting(&mut payload, &jp, &mut diagnostics);
                            had_json_ld = true;
                            diagnostics.push("merged JobPosting from json-ld".into());
                        } else {
                            diagnostics.push(
                                "additional JobPosting json-ld ignored (multi-listing page)".into(),
                            );
                        }
                    }
                }
            }
        }
    }

    if payload.description_plain.is_none() && !unavailable {
        if let Some(plain) = readable_text_from_html(html) {
            let t = plain.trim();
            if !t.is_empty() && !looks_like_embedded_app_dump(&t) {
                let (clamped, truncated) = clamp_chars_with_flag(t, MAX_FALLBACK_DESCRIPTION_CHARS);
                payload.description_plain = Some(clamped);
                if truncated {
                    diagnostics.push(format!(
                        "description_plain truncated to {MAX_FALLBACK_DESCRIPTION_CHARS} characters (see raw snapshot for full page)"
                    ));
                }
                used_fallback = true;
                diagnostics.push("description_plain from structured HTML text".into());
            } else if !t.is_empty() {
                diagnostics.push("rejected fallback text (app bundle / dump heuristics)".into());
            }
        }
    }

    if unavailable {
        payload.title = None;
        payload.employer = None;
        payload.location = None;
        payload.employment_type = None;
        payload.work_mode = None;
        payload.compensation = None;
        payload.description_plain = None;
    } else if let Some(ref desc) = payload.description_plain {
        if looks_like_embedded_app_dump(desc) {
            payload.description_plain = None;
            diagnostics.push("cleared description matching app-dump heuristics".into());
        }
    }

    let outcome = classify_extraction_outcome(&payload, unavailable, had_json_ld, used_fallback);
    diagnostics.insert(0, format!("{DIAG_OUTCOME_PREFIX}{outcome}"));
    if outcome == OUTCOME_SOURCE_UNAVAILABLE {
        diagnostics.push("fill fields manually; raw HTML preserved in snapshot".into());
    } else if outcome == OUTCOME_EXTRACTION_FAILED {
        diagnostics.push("no reliable posting text extracted; use raw snapshot".into());
    } else if outcome == OUTCOME_PARTIAL {
        diagnostics.push("some fields missing; review before marking reviewed".into());
    }

    ExtractPostingResult {
        payload,
        diagnostics,
    }
}

fn classify_extraction_outcome(
    payload: &FetchedPostingPayload,
    unavailable: bool,
    had_json_ld: bool,
    used_fallback: bool,
) -> &'static str {
    if unavailable {
        return OUTCOME_SOURCE_UNAVAILABLE;
    }
    let has_desc = payload
        .description_plain
        .as_ref()
        .is_some_and(|d| !d.trim().is_empty());
    let desc_rich = payload
        .description_plain
        .as_ref()
        .is_some_and(|d| d.chars().count() >= 40);
    let has_title = payload.title.is_some();
    let has_employer = payload.employer.is_some();
    if has_title && has_employer && (desc_rich || (had_json_ld && has_desc)) {
        return OUTCOME_RECOGNIZED;
    }
    if has_title || has_employer || desc_rich || had_json_ld {
        return OUTCOME_PARTIAL;
    }
    if used_fallback {
        return OUTCOME_EXTRACTION_FAILED;
    }
    OUTCOME_EXTRACTION_FAILED
}

pub fn detect_source_unavailable(html: &str) -> bool {
    let probe = source_unavailability_probe_text(html);
    let lower = probe.to_ascii_lowercase();
    const STRONG: &[&str] = &[
        "job not found",
        "may have been taken down",
        "this job is no longer",
        "posting has expired",
        "no longer available",
    ];
    if STRONG.iter().any(|n| lower.contains(n)) {
        return true;
    }
    let title_h1 = title_and_primary_heading_text(html).to_ascii_lowercase();
    title_h1.contains("page not found") || title_h1.contains("404")
}

/// Visible title + primary content only (excludes footer/scripts); avoids incidental site chrome.
fn source_unavailability_probe_text(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut parts: Vec<String> = Vec::new();
    if let Ok(sel) = Selector::parse("title") {
        for el in document.select(&sel) {
            let t: String = el.text().collect();
            let t = t.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        }
    }
    for sel in ["main", "article", "[role='main']", "body"] {
        if let Ok(s) = Selector::parse(sel) {
            if let Some(el) = document.select(&s).next() {
                parts.push(visible_text_from_element(el));
                break;
            }
        }
    }
    parts.join("\n")
}

fn title_and_primary_heading_text(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut parts: Vec<String> = Vec::new();
    if let Ok(sel) = Selector::parse("title") {
        for el in document.select(&sel) {
            parts.push(el.text().collect::<String>());
        }
    }
    for sel in ["main h1", "article h1", "h1"] {
        if let Ok(s) = Selector::parse(sel) {
            if let Some(el) = document.select(&s).next() {
                parts.push(visible_text_from_element(el));
                break;
            }
        }
    }
    parts.join(" ")
}

pub fn looks_like_embedded_app_dump(text: &str) -> bool {
    if text.chars().count() > 50_000 {
        return true;
    }
    const MARKERS: &[&str] = &[
        "af_initdatacallback",
        "wiz_global_data",
        "window.googletag",
        "google.jobs.",
        "_dumpexception",
        "webpackjsonp",
        "__next_data__",
    ];
    let lower = text.to_ascii_lowercase();
    if MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    let fn_count = lower.matches("function(").count();
    lower.matches("{\"@type\"").count() > 3 || fn_count > 25
}

fn readable_text_from_html(html: &str) -> Option<String> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let primary = [
        "main",
        "article",
        "[role='main']",
        "#job-description",
        ".job-description",
        "#content",
    ];
    for sel in primary {
        if let Ok(s) = Selector::parse(sel) {
            for el in document.select(&s) {
                let text = visible_text_from_element(el);
                let trimmed = text.trim();
                if trimmed.chars().count() >= 80 && !looks_like_embedded_app_dump(trimmed) {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    let body_sel = Selector::parse("body").ok()?;
    let body = document.select(&body_sel).next()?;
    let text = visible_text_from_element(body);
    let trimmed = text.trim();
    if trimmed.chars().count() >= 80 {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn visible_text_from_element(root: scraper::ElementRef<'_>) -> String {
    use scraper::{ElementRef, Node};
    let mut lines: Vec<String> = Vec::new();
    fn walk(el: ElementRef<'_>, lines: &mut Vec<String>) {
        for child in el.children() {
            match child.value() {
                Node::Text(t) => {
                    let s = t.trim();
                    if !s.is_empty() {
                        lines.push(s.to_string());
                    }
                }
                Node::Element(e) => {
                    let tag = e.name();
                    if matches!(
                        tag,
                        "script"
                            | "style"
                            | "noscript"
                            | "svg"
                            | "iframe"
                            | "template"
                            | "nav"
                            | "footer"
                            | "header"
                    ) {
                        continue;
                    }
                    if let Some(child_el) = ElementRef::wrap(child) {
                        walk(child_el, lines);
                    }
                }
                _ => {}
            }
        }
    }
    walk(root, &mut lines);
    normalize_visible_lines(&lines)
}

fn normalize_visible_lines(lines: &[String]) -> String {
    let mut out = String::new();
    let mut prev_blank = false;
    for line in lines {
        let compact: String = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if compact.is_empty() {
            if !prev_blank {
                out.push('\n');
                prev_blank = true;
            }
            continue;
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&compact);
        out.push('\n');
        prev_blank = false;
    }
    out.trim().to_string()
}

fn clamp_chars_with_flag(s: &str, max_chars: usize) -> (String, bool) {
    let n = s.chars().count();
    if n <= max_chars {
        (s.to_string(), false)
    } else {
        (s.chars().take(max_chars).collect(), true)
    }
}

fn extract_json_ld_blocks(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(i) = lower[search..].find("application/ld+json") {
        let rel = search + i;
        search = rel + 20;
        if let Some(gt) = html[rel..].find('>') {
            let start = rel + gt + 1;
            if let Some(end) = html[start..].find("</script") {
                let slice = html[start..start + end].trim();
                if !slice.is_empty() {
                    out.push(slice.to_string());
                }
            }
        }
    }
    out
}

fn find_job_posting_value(v: &Value) -> Option<Value> {
    match v {
        Value::Array(a) => {
            for item in a {
                if let Some(jp) = find_job_posting_value(item) {
                    return Some(jp);
                }
            }
            None
        }
        Value::Object(m) => {
            let typ = m.get("@type").and_then(|t| match t {
                Value::String(s) => Some(s.as_str()),
                Value::Array(a) => a.iter().find_map(|x| x.as_str()),
                _ => None,
            });
            if typ == Some("JobPosting")
                || m.get("@type").and_then(|t| t.as_array()).is_some_and(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .any(|s| s == "JobPosting")
                })
            {
                return Some(Value::Object(m.clone()));
            }
            for (_k, val) in m {
                if let Some(jp) = find_job_posting_value(val) {
                    return Some(jp);
                }
            }
            None
        }
        _ => None,
    }
}

fn merge_job_posting(
    payload: &mut FetchedPostingPayload,
    jp: &Value,
    diagnostics: &mut Vec<String>,
) {
    let obj = match jp.as_object() {
        Some(o) => o,
        None => return,
    };

    if let Some(t) = obj.get("title").and_then(|x| x.as_str()) {
        payload.title = Some(t.trim().to_string());
    }
    if let Some(h) = obj.get("hiringOrganization") {
        if let Some(name) = h.get("name").and_then(|x| x.as_str()) {
            payload.employer = Some(name.trim().to_string());
        } else if let Some(s) = h.as_str() {
            payload.employer = Some(s.trim().to_string());
        }
    }
    if let Some(loc) = obj.get("jobLocation") {
        if let Some(addr) = loc.get("address") {
            let mut parts = Vec::new();
            for k in &["addressLocality", "addressRegion", "addressCountry"] {
                if let Some(p) = addr.get(*k).and_then(|x| x.as_str()) {
                    parts.push(p.trim());
                }
            }
            if !parts.is_empty() {
                payload.location = Some(parts.join(", "));
            }
        }
    }
    if let Some(emp) = obj.get("employmentType") {
        match emp {
            Value::String(s) => {
                payload.employment_type = map_employment_schema(s, diagnostics);
            }
            Value::Array(a) => {
                for item in a {
                    if let Some(s) = item.as_str() {
                        let mapped = map_employment_schema(s, diagnostics);
                        if mapped.is_some() {
                            payload.employment_type = mapped;
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let jwl = obj.get("jobLocationType").and_then(|x| x.as_str());
    if let Some(s) = jwl {
        if s.eq_ignore_ascii_case("TELECOMMUTE") {
            payload.work_mode = Some("remote".into());
        }
    }

    if let Some(cr) = obj.get("baseSalary") {
        merge_salary(payload, cr, diagnostics);
    }

    if let Some(desc) = obj.get("description") {
        if let Some(s) = desc.as_str() {
            let plain = plain_text_from_htmlish(s);
            if !plain.is_empty() {
                let (clamped, truncated) =
                    clamp_chars_with_flag(&plain, MAX_FALLBACK_DESCRIPTION_CHARS);
                if truncated {
                    diagnostics.push(format!(
                        "json-ld description truncated to {MAX_FALLBACK_DESCRIPTION_CHARS} characters"
                    ));
                }
                payload.description_plain = Some(clamped);
            }
        }
    }
}

fn plain_text_from_htmlish(fragment: &str) -> String {
    use scraper::Html;
    if fragment.contains('<') {
        let frag = Html::parse_fragment(fragment);
        visible_text_from_element(frag.root_element())
    } else {
        fragment.trim().to_string()
    }
}

fn map_employment_schema(s: &str, diagnostics: &mut Vec<String>) -> Option<String> {
    let u = s.to_ascii_lowercase();
    let out = if u.contains("full") {
        Some("full_time".into())
    } else if u.contains("part") {
        Some("part_time".into())
    } else if u.contains("contract") {
        Some("contract".into())
    } else if u.contains("intern") {
        Some("internship".into())
    } else {
        diagnostics.push(format!("employmentType not mapped: {s}"));
        None
    };
    out
}

fn merge_salary(payload: &mut FetchedPostingPayload, v: &Value, diagnostics: &mut Vec<String>) {
    let obj = match v.as_object() {
        Some(o) => o,
        None => return,
    };
    let unit = obj
        .get("unitText")
        .and_then(|x| x.as_str())
        .map(|s| s.to_ascii_lowercase());
    let mut min = None::<f64>;
    let mut max = None::<f64>;
    if let Some(r) = obj.get("value") {
        if let Some(rv) = r.as_object() {
            min = rv.get("minValue").and_then(|x| x.as_f64());
            max = rv.get("maxValue").and_then(|x| x.as_f64());
        }
    }
    let unit_norm = match unit.as_deref() {
        Some("year") | Some("y") | Some("annual") => Some("annual".into()),
        Some("hour") | Some("h") => Some("hourly".into()),
        Some(other) => {
            diagnostics.push(format!("baseSalary unitText not mapped: {other}"));
            None
        }
        None => None,
    };
    if unit_norm.is_some() || min.is_some() || max.is_some() {
        payload.compensation = Some(crate::Compensation {
            unit: unit_norm,
            min,
            max,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
    }

    #[test]
    fn localhost_url_rejected() {
        assert!(validate_public_fetch_url("http://localhost/x").is_err());
        assert!(validate_public_fetch_url("https://127.0.0.1/").is_err());
    }

    #[test]
    fn resolve_public_socket_addr_rejects_loopback_literal() {
        assert!(resolve_public_socket_addr("127.0.0.1", 443).is_err());
    }

    #[test]
    fn resolve_public_socket_addr_rejects_private_literal() {
        assert!(resolve_public_socket_addr("10.0.0.1", 80).is_err());
    }

    #[test]
    fn extract_json_ld_job_posting() {
        let html = r#"<!DOCTYPE html><html><head>
<script type="application/ld+json">
{
  "@context":"https://schema.org/",
  "@type":"JobPosting",
  "title":"Senior Engineer",
  "hiringOrganization":{"@type":"Organization","name":"Acme Corp"},
  "jobLocation":{"@type":"Place","address":{"addressLocality":"Austin","addressRegion":"TX"}},
  "description":"<p>Build systems.</p>"
}
</script></head><body></body></html>"#;
        let r = extract_from_html(html);
        assert_eq!(r.payload.title.as_deref(), Some("Senior Engineer"));
        assert_eq!(r.payload.employer.as_deref(), Some("Acme Corp"));
        assert!(r
            .payload
            .location
            .as_ref()
            .map(|s| s.contains("Austin"))
            .unwrap_or(false));
        assert!(r
            .payload
            .description_plain
            .as_ref()
            .map(|s| s.contains("Build systems"))
            .unwrap_or(false));
        assert_eq!(
            parse_extraction_outcome(&r.diagnostics).as_deref(),
            Some(OUTCOME_RECOGNIZED)
        );
    }

    #[test]
    fn careers_unavailable_fixture_rejects_junk_and_other_jobs() {
        let html = fixture("careers_search_unavailable.html");
        let r = extract_from_html(&html);
        assert_eq!(
            parse_extraction_outcome(&r.diagnostics).as_deref(),
            Some(OUTCOME_SOURCE_UNAVAILABLE)
        );
        assert!(r.payload.title.is_none());
        assert!(r.payload.employer.is_none());
        let desc = r.payload.description_plain.as_deref().unwrap_or("");
        assert!(!desc.contains("AF_initDataCallback"));
        assert!(!desc.contains("Wrong job"));
        assert!(!desc.contains("Software Engineer II"));
    }

    #[test]
    fn greenhouse_fixture_extracts_title_employer_and_qualifications() {
        let html = fixture("greenhouse_job_posting.html");
        let r = extract_from_html(&html);
        assert_eq!(r.payload.title.as_deref(), Some("Platform Engineer"));
        assert_eq!(r.payload.employer.as_deref(), Some("Fixture Labs"));
        assert!(r
            .payload
            .description_plain
            .as_deref()
            .unwrap_or("")
            .contains("Qualifications"));
        assert_eq!(
            parse_extraction_outcome(&r.diagnostics).as_deref(),
            Some(OUTCOME_RECOGNIZED)
        );
    }

    #[test]
    fn script_body_does_not_enter_fallback_description() {
        let html = r#"<!DOCTYPE html><html><head></head><body>
        <script>function evil(){console.log('secret');}</script>
        <main><p>This is a sufficiently long job description paragraph that should be extracted when no json-ld is present at all on this synthetic fixture page.</p></main>
        </body></html>"#;
        let r = extract_from_html(html);
        let desc = r.payload.description_plain.as_deref().unwrap_or("");
        assert!(!desc.contains("function"));
        assert!(!desc.contains("console.log"));
        assert!(desc.contains("sufficiently long job description"));
    }

    #[test]
    fn valid_page_with_footer_error_text_not_unavailable() {
        let html = r#"<!DOCTYPE html><html><head>
<script type="application/ld+json">
{"@type":"JobPosting","title":"Real Role","hiringOrganization":{"name":"Acme"},
 "description":"<p>We build things. Qualifications include 5 years experience.</p>"}
</script></head><body>
<main><p>Apply for Real Role today.</p></main>
<footer>If you followed a bad link you may see a page not found elsewhere on this site.</footer>
</body></html>"#;
        let r = extract_from_html(html);
        assert_ne!(
            parse_extraction_outcome(&r.diagnostics).as_deref(),
            Some(OUTCOME_SOURCE_UNAVAILABLE),
            "incidental footer copy must not mark whole page unavailable"
        );
        assert_eq!(r.payload.title.as_deref(), Some("Real Role"));
    }

    #[test]
    fn fallback_truncation_is_reported() {
        let big = "x".repeat(30_000);
        let html = format!(r#"<!DOCTYPE html><html><body><main><p>{big}</p></main></body></html>"#);
        let r = extract_from_html(&html);
        assert!(r.diagnostics.iter().any(|d| d.contains("truncated to")));
        assert!(
            r.payload
                .description_plain
                .as_ref()
                .unwrap()
                .chars()
                .count()
                <= MAX_FALLBACK_DESCRIPTION_CHARS
        );
    }

    #[test]
    fn outcome_partial_when_only_title() {
        let html_with_ld = r#"<html><head><script type="application/ld+json">{"@type":"JobPosting","title":"Only Title"}</script></head><body></body></html>"#;
        let r = extract_from_html(html_with_ld);
        assert_eq!(
            parse_extraction_outcome(&r.diagnostics).as_deref(),
            Some(OUTCOME_PARTIAL)
        );
    }
}
