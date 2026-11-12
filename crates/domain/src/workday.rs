//! Workday MyWorkdayJobs CXS job-detail adapter (bounded).

use serde::Deserialize;
use serde_json::Value;

use crate::{
    content_meaningful, greenhouse_content_to_plain, ExtractPostingResult, FetchedPostingPayload,
    DIAG_OUTCOME_PREFIX, OUTCOME_EXTRACTION_FAILED, OUTCOME_PARTIAL, OUTCOME_RECOGNIZED,
    OUTCOME_SOURCE_UNAVAILABLE,
};

pub const WORKDAY_ADAPTER_ID: &str = "workday_cxs_job_detail";
pub const WORKDAY_ADAPTER_VERSION: &str = "1";

pub const RETRIEVAL_KIND_WORKDAY: &str = "workday_cxs_job_detail";

const MYWORKDAYJOBS_SUFFIX: &str = ".myworkdayjobs.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkdayJobRef {
    pub host: String,
    pub tenant: String,
    pub site: String,
    /// Path from `job/` through job slug (no leading slash), e.g. `job/USA---Remote/Title-_R30634`.
    pub job_subpath: String,
    /// Requisition token from URL when present, e.g. `R30634` or `R30636-1`.
    pub url_requisition: Option<String>,
}

fn is_locale_segment(seg: &str) -> bool {
    let Some((a, b)) = seg.split_once('-') else {
        return false;
    };
    a.len() == 2
        && b.len() == 2
        && a.chars().all(|c| c.is_ascii_lowercase())
        && b.chars().all(|c| c.is_ascii_uppercase())
}

fn tenant_from_host(host: &str) -> Option<String> {
    let host_lower = host.to_ascii_lowercase();
    if !host_lower.ends_with(MYWORKDAYJOBS_SUFFIX) {
        return None;
    }
    let stem = host_lower.strip_suffix(MYWORKDAYJOBS_SUFFIX)?;
    let tenant = stem.split('.').next()?;
    if tenant.is_empty() {
        return None;
    }
    Some(tenant.to_string())
}

/// Parse public job-detail URLs on `*.myworkdayjobs.com`.
pub fn parse_workday_job_detail_url(input: &str) -> Option<WorkdayJobRef> {
    let u = url::Url::parse(input.trim()).ok()?;
    if u.scheme() != "https" {
        return None;
    }
    let host = u.host_str()?.to_string();
    let tenant = tenant_from_host(&host)?;
    let mut segs: Vec<&str> = u
        .path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();
    if segs.len() < 3 {
        return None;
    }
    if is_locale_segment(segs[0]) {
        segs.remove(0);
    }
    if segs.len() < 3 {
        return None;
    }
    let site = segs[0].to_string();
    if segs[1] != "job" {
        return None;
    }
    let tail: Vec<&str> = segs[2..].to_vec();
    if tail.is_empty() {
        return None;
    }
    let job_subpath = format!("job/{}", tail.join("/"));
    let url_requisition = extract_url_requisition(&job_subpath);
    Some(WorkdayJobRef {
        host,
        tenant,
        site,
        job_subpath,
        url_requisition,
    })
}

/// Last `_R` + digits + optional `-digits` in the job path.
pub fn extract_url_requisition(job_subpath: &str) -> Option<String> {
    let last_seg = job_subpath.rsplit('/').next()?;
    let idx = last_seg.rfind("_R")?;
    let rest = &last_seg[idx + 1..];
    if rest.is_empty() || !rest.starts_with('R') {
        return None;
    }
    let req: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if req.len() <= 1 {
        return None;
    }
    Some(req)
}

pub fn workday_cxs_job_url(wd: &WorkdayJobRef) -> String {
    format!(
        "https://{}/wday/cxs/{}/{}/{}",
        wd.host, wd.tenant, wd.site, wd.job_subpath
    )
}

pub fn is_workday_cxs_url(input: &str) -> bool {
    url::Url::parse(input.trim())
        .map(|u| u.path().contains("/wday/cxs/"))
        .unwrap_or(false)
}

fn requisition_ids_match(url_req: Option<&str>, json_req: &str) -> bool {
    let Some(url_req) = url_req else {
        return true;
    };
    let json = json_req.trim();
    let url_norm = normalize_requisition(url_req);
    let json_norm = normalize_requisition(json);
    url_norm == json_norm
}

fn normalize_requisition(s: &str) -> String {
    let s = s.trim().trim_start_matches('R');
    let base = s.split('-').next().unwrap_or(s);
    format!("R{}", base)
}

fn map_time_type(time_type: &str) -> Option<String> {
    let t = time_type.trim().to_ascii_lowercase();
    match t.as_str() {
        "full time" | "full-time" => Some("full_time".into()),
        "part time" | "part-time" => Some("part_time".into()),
        _ => None,
    }
}

fn work_mode_from_title(title: &str) -> Option<String> {
    if title.to_ascii_lowercase().contains("(hybrid)") {
        Some("hybrid".into())
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
struct WorkdayCxsRoot {
    #[serde(default, rename = "jobPostingInfo")]
    job_posting_info: Option<WorkdayJobPostingInfo>,
    #[serde(rename = "hiringOrganization")]
    hiring_organization: Option<WorkdayOrg>,
    #[serde(rename = "errorCode")]
    error_code: Option<String>,
    #[serde(rename = "httpStatus")]
    http_status: Option<i64>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkdayOrg {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkdayJobPostingInfo {
    id: Option<String>,
    title: Option<String>,
    #[serde(rename = "jobDescription")]
    job_description: Option<String>,
    location: Option<String>,
    #[serde(rename = "timeType")]
    time_type: Option<String>,
    #[serde(rename = "jobReqId")]
    job_req_id: Option<String>,
    #[serde(rename = "jobPostingId")]
    job_posting_id: Option<String>,
    #[serde(rename = "externalUrl")]
    external_url: Option<String>,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    posted: Option<bool>,
}

fn push_gap(diags: &mut Vec<String>, key: &str) {
    diags.push(format!("gap:{key}"));
}

pub fn workday_http_error_outcome(status: i64) -> (&'static str, Vec<String>) {
    let mut diags = vec![format!("http_status:{status}")];
    let outcome = if status == 404 {
        diags.push("workday:not_found".into());
        OUTCOME_SOURCE_UNAVAILABLE
    } else if (500..600).contains(&status) || status == 429 {
        diags.push("workday:provider_error".into());
        OUTCOME_EXTRACTION_FAILED
    } else if (400..500).contains(&status) {
        diags.push("workday:client_error".into());
        OUTCOME_EXTRACTION_FAILED
    } else {
        OUTCOME_EXTRACTION_FAILED
    };
    diags.insert(0, format!("{DIAG_OUTCOME_PREFIX}{outcome}"));
    (outcome, diags)
}

pub fn interpret_workday_cxs_json(
    raw_json: &str,
    url_requisition: Option<&str>,
) -> ExtractPostingResult {
    let mut diagnostics = Vec::new();
    diagnostics.push(format!("adapter:{WORKDAY_ADAPTER_ID}"));
    diagnostics.push(format!("adapter_version:{WORKDAY_ADAPTER_VERSION}"));

    let root: WorkdayCxsRoot = match serde_json::from_str(raw_json) {
        Ok(r) => r,
        Err(e) => {
            diagnostics.push(format!("retrieval:json_parse:{e}"));
            return outcome_result(
                FetchedPostingPayload::default(),
                OUTCOME_EXTRACTION_FAILED,
                diagnostics,
            );
        }
    };

    if root.job_posting_info.is_none() {
        if root.error_code.is_some() || root.http_status == Some(404) {
            diagnostics.push(format!(
                "workday:error:{}",
                root.message.as_deref().unwrap_or("unknown")
            ));
            return outcome_result(
                FetchedPostingPayload::default(),
                OUTCOME_SOURCE_UNAVAILABLE,
                diagnostics,
            );
        }
        diagnostics.push("workday:missing_job_posting_info".into());
        return outcome_result(
            FetchedPostingPayload::default(),
            OUTCOME_EXTRACTION_FAILED,
            diagnostics,
        );
    }

    let info = root.job_posting_info.unwrap();
    if info.posted == Some(false) {
        diagnostics.push("workday:not_posted".into());
        return outcome_result(
            FetchedPostingPayload::default(),
            OUTCOME_SOURCE_UNAVAILABLE,
            diagnostics,
        );
    }

    if let Some(ref req) = info.job_req_id {
        if !requisition_ids_match(url_requisition, req) {
            diagnostics.push(format!(
                "workday:requisition_mismatch:url={:?}:json={req}",
                url_requisition
            ));
            return outcome_result(
                FetchedPostingPayload::default(),
                OUTCOME_EXTRACTION_FAILED,
                diagnostics,
            );
        }
    }

    let title = info
        .title
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let employer = root
        .hiring_organization
        .as_ref()
        .and_then(|o| o.name.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let location = info
        .location
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let description_plain = info
        .job_description
        .as_deref()
        .map(greenhouse_content_to_plain)
        .filter(|s| !s.is_empty());

    let mut payload = FetchedPostingPayload::default();
    payload.title = title.clone();
    payload.employer = employer.clone();
    payload.location = location;
    payload.description_plain = description_plain.clone();
    payload.first_published = info.start_date.clone();
    payload.application_deadline = info.end_date.clone();

    if let Some(ref tt) = info.time_type {
        if let Some(mapped) = map_time_type(tt) {
            payload.employment_type = Some(mapped);
        } else {
            push_gap(&mut diagnostics, "time_type_unmapped");
        }
    } else {
        push_gap(&mut diagnostics, "time_type");
    }

    if let Some(ref t) = title {
        if let Some(mode) = work_mode_from_title(t) {
            payload.work_mode = Some(mode);
        }
    }

    if info.id.as_deref().unwrap_or("").is_empty() {
        push_gap(&mut diagnostics, "workday_posting_id");
    }
    if info.job_req_id.as_deref().unwrap_or("").is_empty() {
        push_gap(&mut diagnostics, "job_req_id");
    }
    if info.external_url.as_deref().unwrap_or("").is_empty() {
        push_gap(&mut diagnostics, "external_url");
    }

    let title_ok = title.is_some();
    let employer_ok = employer.is_some();
    let desc_ok = description_plain
        .as_ref()
        .is_some_and(|d| content_meaningful(d));

    let outcome = if !title_ok || !employer_ok {
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

pub fn build_workday_provenance(
    wd: &WorkdayJobRef,
    ingest_url: &str,
    cxs_request_url: &str,
    job_posting_id: Option<&str>,
    job_req_id: Option<&str>,
    external_url: Option<&str>,
    workday_internal_id: Option<&str>,
) -> Value {
    serde_json::json!({
        "adapter_id": WORKDAY_ADAPTER_ID,
        "adapter_version": WORKDAY_ADAPTER_VERSION,
        "tenant": wd.tenant,
        "site_id": wd.site,
        "workday_job_posting_id": job_posting_id,
        "job_req_id": job_req_id,
        "url_requisition": wd.url_requisition,
        "external_url": external_url,
        "workday_internal_posting_id": workday_internal_id,
        "ingest_url": ingest_url,
        "provider_request_url": cxs_request_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_crowdstrike_en_us_job_url() {
        let url = "https://crowdstrike.wd5.myworkdayjobs.com/en-US/crowdstrikecareers/job/USA---Remote/Detection-Engineer--Falcon-Complete--Remote-_R30634";
        let wd = parse_workday_job_detail_url(url).expect("parse");
        assert_eq!(wd.tenant, "crowdstrike");
        assert_eq!(wd.site, "crowdstrikecareers");
        assert!(wd.job_subpath.contains("R30634"));
        assert_eq!(wd.url_requisition.as_deref(), Some("R30634"));
        let cxs = workday_cxs_job_url(&wd);
        assert!(cxs.contains("/wday/cxs/crowdstrike/crowdstrikecareers/job/"));
    }

    #[test]
    fn requisition_suffix_with_dash() {
        let path = "job/USA---Redmond-WA/Sr-Software-Engineer-II---Sensor--Linux-Guardian--Hybrid-_R30636-1";
        assert_eq!(extract_url_requisition(path).as_deref(), Some("R30636-1"));
        assert!(requisition_ids_match(Some("R30636-1"), "R30636"));
    }

    #[test]
    fn rejects_non_workday_host() {
        assert!(parse_workday_job_detail_url("https://example.com/job/foo").is_none());
    }

    #[test]
    fn fixture_r30561_recognized() {
        let raw = include_str!("../tests/fixtures/workday/crowdstrike-r30561.json");
        let r = interpret_workday_cxs_json(raw, Some("R30561"));
        assert!(r.diagnostics.iter().any(|d| d.contains("recognized")));
        assert_eq!(
            r.payload.title.as_deref(),
            Some("Data Center Technician (Sacramento, CA)")
        );
        assert_eq!(r.payload.employer.as_deref(), Some("CrowdStrike, Inc."));
        assert!(r
            .payload
            .description_plain
            .as_ref()
            .is_some_and(|d| d.contains("What You'll Do")));
    }

    #[test]
    fn fixture_not_found_unavailable() {
        let raw = include_str!("../tests/fixtures/workday/not-found.json");
        let r = interpret_workday_cxs_json(raw, Some("R99999"));
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.contains(OUTCOME_SOURCE_UNAVAILABLE)));
    }

    #[test]
    fn malformed_json_fails() {
        let r = interpret_workday_cxs_json("{", Some("R1"));
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.contains(OUTCOME_EXTRACTION_FAILED)));
    }
}
