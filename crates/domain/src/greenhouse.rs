//! Greenhouse Job Board API adapter (Milestone M).

use serde::Deserialize;
use serde_json::Value;

use crate::{
    content_meaningful, greenhouse_content_to_plain, ExtractPostingResult, FetchedPostingPayload,
    DIAG_OUTCOME_PREFIX, OUTCOME_EXTRACTION_FAILED, OUTCOME_PARTIAL, OUTCOME_RECOGNIZED,
    OUTCOME_SOURCE_UNAVAILABLE,
};

pub const GREENHOUSE_ADAPTER_ID: &str = "greenhouse_job_board_api";
pub const GREENHOUSE_ADAPTER_VERSION: &str = "1";

pub const RETRIEVAL_KIND_GREENHOUSE: &str = "greenhouse_job_board_api";

const JOB_BOARDS_HOST: &str = "job-boards.greenhouse.io";
const BOARDS_API_HOST: &str = "boards-api.greenhouse.io";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreenhouseJobRef {
    pub board_token: String,
    pub post_id: String,
}

/// v1: `https://job-boards.greenhouse.io/{board}/jobs/{post_id}`
pub fn parse_greenhouse_job_detail_url(input: &str) -> Option<GreenhouseJobRef> {
    let u = url::Url::parse(input.trim()).ok()?;
    if u.scheme() != "https" {
        return None;
    }
    if u.host_str()? != JOB_BOARDS_HOST {
        return None;
    }
    let segs = u
        .path_segments()
        .map(|s| s.collect::<Vec<_>>())
        .unwrap_or_default();
    if segs.len() != 3 {
        return None;
    }
    if segs[1] != "jobs" {
        return None;
    }
    let board = segs[0].trim();
    let post_id = segs[2].trim();
    if board.is_empty() || post_id.is_empty() || !post_id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(GreenhouseJobRef {
        board_token: board.to_string(),
        post_id: post_id.to_string(),
    })
}

pub fn greenhouse_api_job_url(board_token: &str, post_id: &str) -> String {
    format!(
        "https://{BOARDS_API_HOST}/v1/boards/{board_token}/jobs/{post_id}?pay_transparency=true"
    )
}

pub fn is_greenhouse_api_url(input: &str) -> bool {
    url::Url::parse(input.trim())
        .ok()
        .and_then(|u| u.host_str().map(|h| h == BOARDS_API_HOST))
        .unwrap_or(false)
}

#[derive(Debug, Deserialize)]
struct GhLocation {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhJobPost {
    id: i64,
    title: Option<String>,
    company_name: Option<String>,
    location: Option<GhLocation>,
    content: Option<String>,
    absolute_url: Option<String>,
    language: Option<String>,
    internal_job_id: Option<i64>,
    requisition_id: Option<String>,
    application_deadline: Option<String>,
    first_published: Option<String>,
    updated_at: Option<String>,
    pay_input_ranges: Option<Vec<Value>>,
}

fn post_ids_match(requested: &str, json_id: i64) -> bool {
    requested.trim().parse::<i64>().ok() == Some(json_id) || requested.trim() == json_id.to_string()
}

fn push_gap(diags: &mut Vec<String>, key: &str) {
    diags.push(format!("gap:{key}"));
}

/// Interpret archived API JSON (no network).
pub fn interpret_greenhouse_job_json(
    raw_json: &str,
    requested_post_id: &str,
) -> ExtractPostingResult {
    let mut diagnostics = Vec::new();
    diagnostics.push(format!("adapter:{GREENHOUSE_ADAPTER_ID}"));
    diagnostics.push(format!("adapter_version:{GREENHOUSE_ADAPTER_VERSION}"));

    let job: GhJobPost = match serde_json::from_str(raw_json) {
        Ok(j) => j,
        Err(e) => {
            diagnostics.push(format!("retrieval:json_parse:{e}"));
            return outcome_result(
                FetchedPostingPayload::default(),
                OUTCOME_EXTRACTION_FAILED,
                diagnostics,
            );
        }
    };

    if !post_ids_match(requested_post_id, job.id) {
        diagnostics.push("greenhouse:post_id_mismatch".into());
        return outcome_result(
            FetchedPostingPayload::default(),
            OUTCOME_EXTRACTION_FAILED,
            diagnostics,
        );
    }

    let title = job
        .title
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let employer = job
        .company_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let location = job
        .location
        .as_ref()
        .and_then(|l| l.name.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let description_plain = job
        .content
        .as_deref()
        .map(greenhouse_content_to_plain)
        .filter(|s| !s.is_empty());

    let mut payload = FetchedPostingPayload::default();
    payload.title = title.clone();
    payload.employer = employer.clone();
    payload.location = location;
    payload.description_plain = description_plain.clone();
    payload.application_deadline = job.application_deadline.clone();
    payload.first_published = job.first_published.clone();
    payload.posting_updated_at = job.updated_at.clone();
    payload.compensation_ranges = job.pay_input_ranges.clone();
    if let Some(lang) = job.language.as_ref().filter(|s| !s.is_empty()) {
        payload.language = Some(lang.clone());
    }

    if job.internal_job_id.is_none() {
        push_gap(&mut diagnostics, "greenhouse_internal_job_id");
    }
    if job.requisition_id.as_deref().unwrap_or("").is_empty() {
        push_gap(&mut diagnostics, "employer_requisition_id");
    }
    if job.absolute_url.as_deref().unwrap_or("").is_empty() {
        push_gap(&mut diagnostics, "api_absolute_url");
    }
    if job.pay_input_ranges.as_ref().is_none_or(|v| v.is_empty()) {
        push_gap(&mut diagnostics, "pay_input_ranges");
    }

    let post_id_ok = true;
    let title_ok = title.is_some();
    let employer_ok = employer.is_some();
    let desc_ok = description_plain
        .as_ref()
        .is_some_and(|d| content_meaningful(d));

    let outcome = if !post_id_ok {
        OUTCOME_EXTRACTION_FAILED
    } else if !title_ok || !employer_ok {
        if title_ok || employer_ok || desc_ok {
            OUTCOME_PARTIAL
        } else {
            OUTCOME_EXTRACTION_FAILED
        }
    } else if !desc_ok {
        OUTCOME_PARTIAL
    } else {
        OUTCOME_RECOGNIZED
    };

    outcome_result(payload, outcome, diagnostics)
}

fn outcome_result(
    payload: FetchedPostingPayload,
    outcome: &str,
    mut diagnostics: Vec<String>,
) -> ExtractPostingResult {
    diagnostics.insert(0, format!("{DIAG_OUTCOME_PREFIX}{outcome}"));
    ExtractPostingResult {
        payload,
        diagnostics,
    }
}

pub fn build_greenhouse_provenance(
    board_token: &str,
    requested_post_id: &str,
    ingest_url: &str,
    provider_request_url: &str,
    api_absolute_url: Option<&str>,
    internal_job_id: Option<i64>,
    requisition_id: Option<&str>,
) -> Value {
    serde_json::json!({
        "adapter_id": GREENHOUSE_ADAPTER_ID,
        "adapter_version": GREENHOUSE_ADAPTER_VERSION,
        "board_token": board_token,
        "greenhouse_post_id": requested_post_id,
        "greenhouse_internal_job_id": internal_job_id,
        "employer_requisition_id": requisition_id,
        "ingest_url": ingest_url,
        "provider_request_url": provider_request_url,
        "api_absolute_url": api_absolute_url,
    })
}

pub fn extraction_outcome_usable(outcome: Option<&str>) -> bool {
    matches!(outcome, Some(OUTCOME_RECOGNIZED) | Some(OUTCOME_PARTIAL))
}

/// Classify non-2xx Greenhouse Job Board API responses (deterministic).
pub fn greenhouse_http_error_outcome(status: i64) -> (&'static str, Vec<String>) {
    let mut diags = vec![format!("http_status:{status}")];
    match status {
        404 => {
            diags.push(format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_SOURCE_UNAVAILABLE}"));
            (OUTCOME_SOURCE_UNAVAILABLE, diags)
        }
        429 => {
            diags.push("retrieval:rate_limited".into());
            diags.push(format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_EXTRACTION_FAILED}"));
            (OUTCOME_EXTRACTION_FAILED, diags)
        }
        500..=599 => {
            diags.push("retrieval:server_error".into());
            diags.push(format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_EXTRACTION_FAILED}"));
            (OUTCOME_EXTRACTION_FAILED, diags)
        }
        400..=499 => {
            diags.push("retrieval:client_error".into());
            diags.push(format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_EXTRACTION_FAILED}"));
            (OUTCOME_EXTRACTION_FAILED, diags)
        }
        _ => {
            diags.push("retrieval:http_error".into());
            diags.push(format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_EXTRACTION_FAILED}"));
            (OUTCOME_EXTRACTION_FAILED, diags)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/greenhouse")
            .join(name);
        fs::read_to_string(&p).unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    #[test]
    fn parse_job_board_url() {
        let r = parse_greenhouse_job_detail_url(
            "https://job-boards.greenhouse.io/flexport/jobs/8009198",
        )
        .unwrap();
        assert_eq!(r.board_token, "flexport");
        assert_eq!(r.post_id, "8009198");
    }

    #[test]
    fn flexport_fixture_recognized() {
        let raw = fixture("flexport-8009198.json");
        let r = interpret_greenhouse_job_json(&raw, "8009198");
        assert_eq!(
            r.diagnostics
                .iter()
                .find(|d| d.starts_with(DIAG_OUTCOME_PREFIX))
                .map(|s| s.strip_prefix(DIAG_OUTCOME_PREFIX).unwrap()),
            Some(OUTCOME_RECOGNIZED)
        );
        assert_eq!(r.payload.title.as_deref(), Some("Account Executive"));
        assert_eq!(r.payload.employer.as_deref(), Some("Flexport"));
    }

    #[test]
    fn post_id_mismatch_fails() {
        let raw = fixture("flexport-8009198.json");
        let r = interpret_greenhouse_job_json(&raw, "999");
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d == "greenhouse:post_id_mismatch"));
    }
}
