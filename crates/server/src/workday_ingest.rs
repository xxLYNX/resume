use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use domain::{
    build_workday_provenance, interpret_workday_cxs_json, parse_extraction_outcome,
    parse_workday_job_detail_url, sha256_hex, snapshot_body_for_storage, workday_cxs_job_url,
    workday_http_error_outcome, WorkdayJobRef, DIAG_OUTCOME_PREFIX, OUTCOME_EXTRACTION_FAILED,
    OUTCOME_SOURCE_UNAVAILABLE, RETRIEVAL_KIND_WORKDAY, WORKDAY_ADAPTER_ID,
    WORKDAY_ADAPTER_VERSION,
};
use serde_json::{json, Value};
use store::Store;

use crate::opportunities::{anyhow_response, IngestPostingResponse};
use crate::AppState;

fn attempt_body_stored(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let (s, _) = snapshot_body_for_storage(bytes);
    Some(s)
}

pub async fn try_ingest_workday(
    state: &Arc<AppState>,
    ingest_url: &str,
    wd: WorkdayJobRef,
    apps_before: i64,
) -> Option<axum::response::Response> {
    let cxs_url = workday_cxs_job_url(&wd);
    let fetcher = Arc::clone(&state.posting_fetcher);
    let cxs_url_fetch = cxs_url.clone();
    let fetch =
        tokio::task::spawn_blocking(move || fetcher.fetch_posting_document(&cxs_url_fetch)).await;
    let fetched = match fetch {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            let mut diags = vec![
                format!("retrieval:transport:{e}"),
                format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_EXTRACTION_FAILED}"),
                "http_status:0".into(),
            ];
            return match Store::open(&state.db_path).and_then(|store| {
                let detail = store.ingest_provider_transport_failure(
                    ingest_url,
                    &cxs_url,
                    &diags,
                    RETRIEVAL_KIND_WORKDAY,
                )?;
                let apps_after = store.count_applications()?;
                Ok((detail, apps_after))
            }) {
                Ok((posting, apps_after)) if apps_after == apps_before => Some(
                    (StatusCode::CREATED, Json(IngestPostingResponse { posting })).into_response(),
                ),
                Ok(_) => Some(
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error":"ingestion must not create applications"})),
                    )
                        .into_response(),
                ),
                Err(e) => Some(anyhow_response(e).into_response()),
            };
        }
        Err(e) => {
            return Some(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("fetch task: {e}")})),
                )
                    .into_response(),
            );
        }
    };

    let body_bytes = fetched.body;
    let chain = serde_json::to_value(&fetched.redirect_chain).unwrap_or(json!([]));
    let status = fetched.status;

    if status < 200 || status >= 300 {
        let (outcome, mut diags) = workday_http_error_outcome(status);
        if let Some(s) = attempt_body_stored(&body_bytes).as_ref() {
            diags.extend(interpret_workday_cxs_json(s, wd.url_requisition.as_deref()).diagnostics);
        }
        let final_outcome = parse_extraction_outcome(&diags).unwrap_or_else(|| outcome.into());
        return persist_bytes(
            state,
            ingest_url,
            &cxs_url,
            &fetched.final_url,
            &chain,
            status,
            &fetched.content_type,
            None,
            &diags,
            &domain::FetchedPostingPayload::default(),
            final_outcome.as_str(),
            attempt_body_stored(&body_bytes).as_deref(),
            apps_before,
            "{}",
            &wd,
        )
        .await;
    }

    if body_bytes.is_empty() {
        let mut diags = vec![
            "http_status:200".into(),
            "retrieval:empty_body".into(),
            format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_EXTRACTION_FAILED}"),
        ];
        return persist_bytes(
            state,
            ingest_url,
            &cxs_url,
            &fetched.final_url,
            &chain,
            status,
            &fetched.content_type,
            None,
            &diags,
            &domain::FetchedPostingPayload::default(),
            OUTCOME_EXTRACTION_FAILED,
            None,
            apps_before,
            "{}",
            &wd,
        )
        .await;
    }

    let (stored_body, body_was_base64) = snapshot_body_for_storage(&body_bytes);
    let hash = sha256_hex(&body_bytes);
    let body_utf8 = if body_was_base64 {
        None
    } else {
        Some(stored_body.as_str())
    };

    let mut extracted = body_utf8
        .map(|s| interpret_workday_cxs_json(s, wd.url_requisition.as_deref()))
        .unwrap_or_else(|| {
            let diags = vec![
                "retrieval:invalid_utf8".into(),
                format!("{DIAG_OUTCOME_PREFIX}{OUTCOME_EXTRACTION_FAILED}"),
            ];
            domain::ExtractPostingResult {
                payload: domain::FetchedPostingPayload::default(),
                diagnostics: diags,
            }
        });

    if body_was_base64 {
        extracted.diagnostics.push("snapshot_body:base64".into());
    }

    let outcome = parse_extraction_outcome(&extracted.diagnostics)
        .unwrap_or_else(|| OUTCOME_EXTRACTION_FAILED.to_string());

    let job_val: Value = body_utf8
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(json!({}));
    let info = job_val.get("jobPostingInfo");
    let provenance = build_workday_provenance(
        &wd,
        ingest_url,
        &cxs_url,
        info.and_then(|i| i.get("jobPostingId").and_then(|v| v.as_str())),
        info.and_then(|i| i.get("jobReqId").and_then(|v| v.as_str())),
        info.and_then(|i| i.get("externalUrl").and_then(|v| v.as_str())),
        info.and_then(|i| i.get("id").and_then(|v| v.as_str())),
    );
    let prov_s = serde_json::to_string(&provenance).unwrap_or_else(|_| "{}".into());

    persist_bytes(
        state,
        ingest_url,
        &cxs_url,
        &fetched.final_url,
        &chain,
        status,
        &fetched.content_type,
        Some((&body_bytes, &hash, &stored_body)),
        &extracted.diagnostics,
        &extracted.payload,
        outcome.as_str(),
        None,
        apps_before,
        &prov_s,
        &wd,
    )
    .await
}

async fn persist_bytes(
    state: &Arc<AppState>,
    ingest_url: &str,
    cxs_url: &str,
    final_url: &str,
    chain: &Value,
    status: i64,
    content_type: &str,
    snapshot: Option<(&[u8], &str, &str)>,
    diagnostics: &[String],
    machine: &domain::FetchedPostingPayload,
    outcome: &str,
    attempt_body: Option<&str>,
    apps_before: i64,
    provenance: &str,
    _wd: &WorkdayJobRef,
) -> Option<axum::response::Response> {
    match Store::open(&state.db_path).and_then(|store| {
        let (snap_bytes, snap_hash, snap_stored) = match snapshot {
            Some((b, h, s)) => (Some(b), Some(h), Some(s)),
            None => (None, None, None),
        };
        let detail = store.ingest_provider_fetch(
            ingest_url,
            cxs_url,
            final_url,
            chain,
            status,
            content_type,
            snap_stored,
            snap_bytes,
            snap_hash,
            provenance,
            WORKDAY_ADAPTER_ID,
            WORKDAY_ADAPTER_VERSION,
            diagnostics,
            machine,
            outcome,
            attempt_body,
            RETRIEVAL_KIND_WORKDAY,
        )?;
        let apps_after = store.count_applications()?;
        Ok((detail, apps_after))
    }) {
        Ok((posting, apps_after)) if apps_after == apps_before => {
            Some((StatusCode::CREATED, Json(IngestPostingResponse { posting })).into_response())
        }
        Ok(_) => Some(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"ingestion must not create applications"})),
            )
                .into_response(),
        ),
        Err(e) => Some(anyhow_response(e).into_response()),
    }
}

pub fn parse_workday_url(url: &str) -> Option<WorkdayJobRef> {
    parse_workday_job_detail_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        workday_http_error_outcome, OUTCOME_EXTRACTION_FAILED, OUTCOME_SOURCE_UNAVAILABLE,
    };

    #[test]
    fn http_404_is_source_unavailable() {
        let (o, d) = workday_http_error_outcome(404);
        assert_eq!(o, OUTCOME_SOURCE_UNAVAILABLE);
        assert!(d.iter().any(|x| x.contains("http_status:404")));
    }

    #[test]
    fn http_503_is_extraction_failed() {
        let (o, _) = workday_http_error_outcome(503);
        assert_eq!(o, OUTCOME_EXTRACTION_FAILED);
    }
}
