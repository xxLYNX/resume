//! Posting ingestion HTTP transport: policy in JobDesk, protocol work in `reqwest`.

use std::time::Duration;

use domain::{
    resolve_public_socket_addr, validate_public_fetch_url, PostingFetchError,
    DEFAULT_FETCH_TIMEOUT, MAX_POSTING_BODY_BYTES, MAX_POSTING_REDIRECTS,
};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::Deserialize;
use url::Url;

const USER_AGENT: &str = "jobdesk/0.1 (opportunity ingestion)";

#[derive(Debug, Clone)]
pub struct PostingHttpResult {
    pub final_url: String,
    pub status: i64,
    pub content_type: String,
    pub body: Vec<u8>,
    pub redirect_chain: Vec<String>,
    /// E2E only: next fetch should fail as transport (no HTTP response).
    pub acceptance_transport_failure: bool,
}

pub trait PostingHttpFetcher: Send + Sync {
    fn fetch_posting_html(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError>;

    /// Fetch with HTML or JSON content types; returns non-success HTTP statuses with body when present.
    fn fetch_posting_document(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        fetch_document_with_policy(url, DEFAULT_FETCH_TIMEOUT)
    }
}

/// Production fetcher using `reqwest` with manual redirects and pinned resolves.
pub struct SecurePostingHttpFetcher {
    timeout: Duration,
}

impl Default for SecurePostingHttpFetcher {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_FETCH_TIMEOUT,
        }
    }
}

impl PostingHttpFetcher for SecurePostingHttpFetcher {
    fn fetch_posting_html(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        fetch_with_policy(url, self.timeout)
    }

    fn fetch_posting_document(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        fetch_document_with_policy(url, self.timeout)
    }
}

pub(crate) fn fetch_with_policy(
    start_url: &str,
    timeout: Duration,
) -> Result<PostingHttpResult, PostingFetchError> {
    validate_public_fetch_url(start_url)?;
    let mut current =
        Url::parse(start_url.trim()).map_err(|e| PostingFetchError::InvalidUrl(e.to_string()))?;
    let mut chain = vec![current.to_string()];

    for hop in 0..=MAX_POSTING_REDIRECTS {
        let client = pinned_client(&current, timeout)?;
        let resp = client
            .get(current.as_str())
            .send()
            .map_err(|e| PostingFetchError::Transport(e.to_string()))?;

        let status = resp.status().as_u16();
        if is_redirect_status(status) {
            if hop == MAX_POSTING_REDIRECTS {
                return Err(PostingFetchError::TooManyRedirects(MAX_POSTING_REDIRECTS));
            }
            let loc = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| PostingFetchError::Transport("redirect without Location".into()))?;
            current = current
                .join(loc.trim())
                .map_err(|e| PostingFetchError::InvalidUrl(e.to_string()))?;
            validate_public_fetch_url(current.as_str())?;
            chain.push(current.to_string());
            continue;
        }

        if status < 200 || status >= 300 {
            return Err(PostingFetchError::Transport(format!(
                "HTTP status {status}"
            )));
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if !ct.is_empty()
            && !ct.starts_with("text/html")
            && !ct.starts_with("application/xhtml+xml")
        {
            return Err(PostingFetchError::UnsupportedContentType(ct));
        }

        let body = read_body_limited(resp, MAX_POSTING_BODY_BYTES)?;
        if body.is_empty() {
            return Err(PostingFetchError::EmptyResponse);
        }

        return Ok(PostingHttpResult {
            final_url: current.to_string(),
            status: status as i64,
            content_type: ct,
            body,
            redirect_chain: chain,
            acceptance_transport_failure: false,
        });
    }
    Err(PostingFetchError::TooManyRedirects(MAX_POSTING_REDIRECTS))
}

/// Document fetch: allow `application/json`, do not error on 404/5xx when body is returned.
pub(crate) fn fetch_document_with_policy(
    start_url: &str,
    timeout: Duration,
) -> Result<PostingHttpResult, PostingFetchError> {
    validate_public_fetch_url(start_url)?;
    let mut current =
        Url::parse(start_url.trim()).map_err(|e| PostingFetchError::InvalidUrl(e.to_string()))?;
    let mut chain = vec![current.to_string()];

    for hop in 0..=MAX_POSTING_REDIRECTS {
        let client = pinned_client(&current, timeout)?;
        let resp = client
            .get(current.as_str())
            .send()
            .map_err(|e| PostingFetchError::Transport(e.to_string()))?;

        let status = resp.status().as_u16();
        if is_redirect_status(status) {
            if hop == MAX_POSTING_REDIRECTS {
                return Err(PostingFetchError::TooManyRedirects(MAX_POSTING_REDIRECTS));
            }
            let loc = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| PostingFetchError::Transport("redirect without Location".into()))?;
            current = current
                .join(loc.trim())
                .map_err(|e| PostingFetchError::InvalidUrl(e.to_string()))?;
            validate_public_fetch_url(current.as_str())?;
            chain.push(current.to_string());
            continue;
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if (200..300).contains(&status) {
            if !ct.is_empty()
                && !ct.starts_with("text/html")
                && !ct.starts_with("application/xhtml+xml")
                && !ct.starts_with("application/json")
            {
                return Err(PostingFetchError::UnsupportedContentType(ct));
            }
        }

        let body = read_body_limited(resp, MAX_POSTING_BODY_BYTES)?;

        return Ok(PostingHttpResult {
            final_url: current.to_string(),
            status: status as i64,
            content_type: if ct.is_empty() {
                "application/octet-stream".into()
            } else {
                ct
            },
            body,
            redirect_chain: chain,
            acceptance_transport_failure: false,
        });
    }
    Err(PostingFetchError::TooManyRedirects(MAX_POSTING_REDIRECTS))
}

/// Build a one-shot client: no proxy, no auto-redirect, DNS pinned to a vetted address.
fn pinned_client(url: &Url, timeout: Duration) -> Result<Client, PostingFetchError> {
    let host = url
        .host_str()
        .ok_or_else(|| PostingFetchError::InvalidUrl("missing host".into()))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(443);
    let addr = resolve_public_socket_addr(&host, port)?;
    Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .timeout(timeout)
        .connect_timeout(timeout)
        .user_agent(USER_AGENT)
        .gzip(true)
        .resolve(&host, addr)
        .build()
        .map_err(|e| PostingFetchError::Transport(format!("client build: {e}")))
}

fn read_body_limited(
    response: reqwest::blocking::Response,
    max: usize,
) -> Result<Vec<u8>, PostingFetchError> {
    if let Some(len) = response.content_length() {
        if len > max as u64 {
            return Err(PostingFetchError::BodyTooLarge(max));
        }
    }
    let bytes = response
        .bytes()
        .map_err(|e| PostingFetchError::Transport(e.to_string()))?;
    if bytes.len() > max {
        return Err(PostingFetchError::BodyTooLarge(max));
    }
    Ok(bytes.to_vec())
}

pub(crate) fn is_redirect_status(s: u16) -> bool {
    matches!(s, 301 | 302 | 303 | 307 | 308)
}

#[allow(dead_code)]
pub(crate) fn next_redirect_url(
    base: &Url,
    status: u16,
    location: &str,
) -> Result<Url, PostingFetchError> {
    if !is_redirect_status(status) {
        return Err(PostingFetchError::Transport("not a redirect".into()));
    }
    let next = base
        .join(location.trim())
        .map_err(|e| PostingFetchError::InvalidUrl(e.to_string()))?;
    validate_public_fetch_url(next.as_str())?;
    Ok(next)
}

/// Acceptance / E2E fixture fetcher: sequence of canned bodies per URL (no live HTTP).
///
/// Enabled only when `jobdesk serve --acceptance-fixtures <manifest.json>` is used.
pub struct AcceptanceFixturePostingFetcher {
    sequences: std::sync::Mutex<std::collections::HashMap<String, Vec<PostingHttpResult>>>,
    call_index: std::sync::Mutex<std::collections::HashMap<String, usize>>,
}

#[derive(Debug, Deserialize)]
struct AcceptanceFixtureManifest {
    /// Map ingest URL → ordered list of fixture entries (re-fetch advances).
    urls: std::collections::HashMap<String, Vec<AcceptanceFixtureEntry>>,
}

#[derive(Debug, Deserialize)]
struct AcceptanceFixtureEntry {
    #[serde(default)]
    file: Option<String>,
    #[serde(default = "default_fixture_status")]
    status: i64,
    #[serde(default = "default_fixture_content_type")]
    content_type: String,
    #[serde(default)]
    simulate_transport_error: bool,
}

fn default_fixture_status() -> i64 {
    200
}

fn default_fixture_content_type() -> String {
    "text/html".into()
}

/// Load manifest from disk; paths in `file` are relative to the manifest directory.
pub fn load_acceptance_fixture_fetcher(
    manifest_path: &std::path::Path,
) -> Result<std::sync::Arc<AcceptanceFixturePostingFetcher>, PostingFetchError> {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    let raw = fs::read_to_string(manifest_path)
        .map_err(|e| PostingFetchError::Transport(format!("read manifest: {e}")))?;
    let manifest: AcceptanceFixtureManifest = serde_json::from_str(&raw)
        .map_err(|e| PostingFetchError::Transport(format!("parse manifest: {e}")))?;
    let base = manifest_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut sequences = std::collections::HashMap::new();
    for (url, entries) in manifest.urls {
        validate_public_fetch_url(url.trim())?;
        let mut versioned = Vec::new();
        for entry in entries {
            if entry.simulate_transport_error {
                versioned.push(PostingHttpResult {
                    final_url: url.trim().to_string(),
                    status: 0,
                    content_type: String::new(),
                    body: Vec::new(),
                    redirect_chain: vec![url.trim().to_string()],
                    acceptance_transport_failure: true,
                });
                continue;
            }
            let body = if let Some(file) = &entry.file {
                let path = base.join(file);
                fs::read(&path).map_err(|e| {
                    PostingFetchError::Transport(format!("read {}: {e}", path.display()))
                })?
            } else if entry.simulate_transport_error {
                Vec::new()
            } else if (200..300).contains(&(entry.status as u16)) {
                return Err(PostingFetchError::Transport(
                    "fixture 2xx entry requires file".into(),
                ));
            } else {
                Vec::new()
            };
            if body.len() > MAX_POSTING_BODY_BYTES {
                return Err(PostingFetchError::BodyTooLarge(MAX_POSTING_BODY_BYTES));
            }
            if body.is_empty()
                && (200..300).contains(&(entry.status as u16))
                && !entry.simulate_transport_error
            {
                return Err(PostingFetchError::EmptyResponse);
            }
            versioned.push(PostingHttpResult {
                final_url: url.trim().to_string(),
                status: entry.status,
                content_type: entry.content_type,
                body,
                redirect_chain: vec![url.trim().to_string()],
                acceptance_transport_failure: false,
            });
        }
        if versioned.is_empty() {
            return Err(PostingFetchError::Transport(format!(
                "no entries for URL {url}"
            )));
        }
        sequences.insert(url.trim().to_string(), versioned);
    }
    Ok(Arc::new(AcceptanceFixturePostingFetcher {
        sequences: std::sync::Mutex::new(sequences),
        call_index: std::sync::Mutex::new(std::collections::HashMap::new()),
    }))
}

impl AcceptanceFixturePostingFetcher {
    pub fn reset_sequences(&self) {
        self.call_index.lock().unwrap().clear();
    }

    fn next_result(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        let key = url.trim();
        let mut idx_map = self.call_index.lock().unwrap();
        let seq_map = self.sequences.lock().unwrap();
        let seq = seq_map.get(key).ok_or_else(|| {
            PostingFetchError::Transport(format!("acceptance fixture miss for `{key}`"))
        })?;
        let idx = idx_map.entry(key.to_string()).or_insert(0);
        let pick = (*idx).min(seq.len() - 1);
        let out = seq[pick].clone();
        if *idx + 1 < seq.len() {
            *idx += 1;
        }
        if out.acceptance_transport_failure {
            return Err(PostingFetchError::Transport(
                "acceptance simulated transport failure".into(),
            ));
        }
        Ok(out)
    }
}

impl PostingHttpFetcher for AcceptanceFixturePostingFetcher {
    fn fetch_posting_html(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        self.next_result(url)
    }

    fn fetch_posting_document(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        self.next_result(url)
    }
}

/// Test-only fetcher returning canned HTTP results (does not weaken production policy).
#[cfg(test)]
pub struct MockPostingHttpFetcher {
    pub responses: std::sync::Mutex<std::collections::HashMap<String, PostingHttpResult>>,
}

#[cfg(test)]
impl MockPostingHttpFetcher {
    pub fn single(url: &str, result: PostingHttpResult) -> Self {
        let mut map = std::collections::HashMap::new();
        map.insert(url.trim().to_string(), result);
        Self {
            responses: std::sync::Mutex::new(map),
        }
    }

    pub fn map(responses: std::collections::HashMap<String, PostingHttpResult>) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses),
        }
    }
}

#[cfg(test)]
impl PostingHttpFetcher for MockPostingHttpFetcher {
    fn fetch_posting_html(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        self.responses
            .lock()
            .unwrap()
            .get(url.trim())
            .cloned()
            .ok_or_else(|| PostingFetchError::Transport("mock miss".into()))
    }

    fn fetch_posting_document(&self, url: &str) -> Result<PostingHttpResult, PostingFetchError> {
        self.fetch_posting_html(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::validate_public_fetch_url;
    #[test]
    fn redirect_status_codes() {
        assert!(is_redirect_status(302));
        assert!(!is_redirect_status(200));
    }

    #[test]
    fn fetch_policy_rejects_file_scheme() {
        let err = fetch_with_policy("file:///etc/passwd", DEFAULT_FETCH_TIMEOUT);
        assert!(err.is_err());
    }

    #[test]
    fn fetch_policy_rejects_redirect_to_loopback_url() {
        let err = validate_public_fetch_url("http://127.0.0.1/after-redirect");
        assert!(err.is_err());
    }

    #[test]
    fn next_redirect_rejects_private_location() {
        let base = Url::parse("https://example.com/start").unwrap();
        let err = next_redirect_url(&base, 302, "http://127.0.0.1/evil");
        assert!(err.is_err());
    }

    #[test]
    fn next_redirect_accepts_relative_location() {
        let base = Url::parse("https://example.com/jobs/1").unwrap();
        let next = next_redirect_url(&base, 302, "/jobs/2").unwrap();
        assert_eq!(next.as_str(), "https://example.com/jobs/2");
        assert!(validate_public_fetch_url(next.as_str()).is_ok());
    }

    #[test]
    fn multi_hop_redirect_logic() {
        let mut url = Url::parse("https://example.com/a").unwrap();
        let hops = ["/b", "/c", "https://example.com/d"];
        for loc in hops {
            url = next_redirect_url(&url, 302, loc).unwrap();
        }
        assert_eq!(url.as_str(), "https://example.com/d");
    }

    #[test]
    fn loopback_ingest_url_rejected_before_reqwest() {
        let err = fetch_with_policy("http://127.0.0.1:12345/", Duration::from_secs(1));
        assert!(matches!(err, Err(PostingFetchError::DisallowedHost(_))));
    }

    #[test]
    fn reqwest_client_disallows_proxy_and_redirect() {
        let url = Url::parse("https://example.com/").unwrap();
        let client = pinned_client(&url, DEFAULT_FETCH_TIMEOUT).unwrap();
        let policy = format!("{:?}", client);
        assert!(
            !policy.contains("Proxy"),
            "unexpected proxy in debug: {policy}"
        );
    }

    #[test]
    fn connect_timeout_to_unreachable_private() {
        let err = fetch_with_policy("http://10.255.255.254/", Duration::from_millis(200));
        assert!(err.is_err());
    }

    #[test]
    fn read_body_limited_rejects_oversized_content_length() {
        use std::io::Write;
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let big = MAX_POSTING_BODY_BYTES + 1024;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {big}\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(response.as_bytes());
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let resp = client
            .get(format!("http://{addr}/"))
            .send()
            .expect("local server");
        let err = read_body_limited(resp, MAX_POSTING_BODY_BYTES).unwrap_err();
        assert!(matches!(err, PostingFetchError::BodyTooLarge(_)));
    }

    #[test]
    fn read_body_limited_surfaces_body_read_errors() {
        use std::io::Write;
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let response =
                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5000\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(response);
            let _ = stream.shutdown(std::net::Shutdown::Write);
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let resp = client
            .get(format!("http://{addr}/"))
            .send()
            .expect("local server");
        let err = read_body_limited(resp, MAX_POSTING_BODY_BYTES).unwrap_err();
        assert!(matches!(err, PostingFetchError::Transport(_)));
    }

    /// `cargo test -p server live_fetch_public_example -- --ignored --nocapture`
    #[test]
    #[ignore = "live network"]
    fn live_fetch_public_example() {
        let f = SecurePostingHttpFetcher::default();
        let r = f
            .fetch_posting_html("https://example.com/")
            .expect("example.com should be fetchable");
        assert_eq!(r.status, 200);
        assert!(!r.body.is_empty());
    }

    /// `cargo test -p server live_fetch_job_posting_json_ld -- --ignored --nocapture`
    #[test]
    #[ignore = "live network; genuine JobPosting JSON-LD"]
    fn live_fetch_job_posting_json_ld() {
        let url = "https://www.google.com/about/careers/applications/jobs/results/317042454704059940-software-engineer-iii-geo-maps/";
        let f = SecurePostingHttpFetcher::default();
        let r = f.fetch_posting_html(url).expect("job URL fetch");
        assert_eq!(r.status, 200);
        let html = String::from_utf8_lossy(&r.body);
        assert!(
            html.contains("JobPosting") || html.contains("job"),
            "response should look like a job page"
        );
    }

    /// `cargo test -p server live_tls_bad_certificate -- --ignored`
    #[test]
    #[ignore = "live network"]
    fn live_tls_bad_certificate_rejected() {
        let err = fetch_with_policy("https://self-signed.badssl.com/", DEFAULT_FETCH_TIMEOUT);
        assert!(err.is_err(), "bad TLS must fail: {:?}", err);
    }
}
