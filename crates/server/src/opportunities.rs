use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use domain::{extract_from_html, sha256_hex, PostingFetchError, POSTING_EXTRACTOR_VERSION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use store::{PostingDetail, Store, UpdatePostingInterpretationRequest};

use crate::posting_http::PostingHttpFetcher;
use crate::AppState;

const EXTRACTOR_ID_JSON_LD: &str = "json_ld_html_v1";

#[derive(Debug, Deserialize)]
pub struct IngestPostingBody {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct IngestPostingResponse {
    pub posting: PostingDetail,
}

pub(crate) fn fetch_posting_via(
    fetcher: &dyn PostingHttpFetcher,
    url: &str,
) -> Result<(String, i64, String, String, Vec<u8>, Vec<String>), PostingFetchError> {
    let fetched = fetcher.fetch_posting_html(url)?;
    let html = String::from_utf8_lossy(&fetched.body).into_owned();
    Ok((
        fetched.final_url,
        fetched.status,
        fetched.content_type,
        html,
        fetched.body,
        fetched.redirect_chain,
    ))
}

fn err_response(err: PostingFetchError) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": err.to_string()})),
    )
}

pub(crate) fn anyhow_response(err: anyhow::Error) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": format!("{err:#}")})),
    )
}

pub async fn ingest_posting(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IngestPostingBody>,
) -> impl IntoResponse {
    if !state.db_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "catalog database missing; run jobdesk import"})),
        )
            .into_response();
    }

    let apps_before = match Store::open(&state.db_path).and_then(|s| s.count_applications()) {
        Ok(n) => n,
        Err(e) => return anyhow_response(e).into_response(),
    };

    let url = body.url.trim().to_string();

    if let Some(gh) = crate::greenhouse_ingest::parse_gh_url(&url) {
        if let Some(resp) =
            crate::greenhouse_ingest::try_ingest_greenhouse(&state, &url, gh, apps_before).await
        {
            return resp;
        }
    }

    let fetcher = Arc::clone(&state.posting_fetcher);
    let fetch_outcome =
        tokio::task::spawn_blocking(move || fetch_posting_via(fetcher.as_ref(), &url)).await;
    let fetch_outcome = match fetch_outcome {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("fetch task: {e}")})),
            )
                .into_response();
        }
    };

    let (final_url, status, content_type, html, bytes, chain) = match fetch_outcome {
        Ok(x) => x,
        Err(e) => return err_response(e).into_response(),
    };

    let extracted = extract_from_html(&html);
    let mut diagnostics = extracted.diagnostics;
    diagnostics.push(format!("http_status:{status}"));
    diagnostics.push(format!("content_type:{content_type}"));

    let hash = sha256_hex(&bytes);
    let machine = extracted.payload;

    match Store::open(&state.db_path).and_then(|store| {
        let chain_json = serde_json::to_value(&chain).unwrap_or(json!([]));
        let detail = store.save_posting_fetch_extended(
            body.url.trim(),
            &final_url,
            &chain_json,
            status,
            &content_type,
            &html,
            &hash,
            EXTRACTOR_ID_JSON_LD,
            POSTING_EXTRACTOR_VERSION,
            &diagnostics,
            &machine,
            "http_html",
            "{}",
            Some(body.url.trim()),
        )?;
        let apps_after = store.count_applications()?;
        Ok((detail, apps_after))
    }) {
        Ok((posting, apps_after)) => {
            if apps_after != apps_before {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":"ingestion must not create applications"})),
                )
                    .into_response();
            }
            (StatusCode::CREATED, Json(IngestPostingResponse { posting })).into_response()
        }
        Err(e) => anyhow_response(e).into_response(),
    }
}

/// Workspace directory names allowed for acceptance reset (not production `workspace/`).
pub fn acceptance_e2e_workspace(workspace_dir: &std::path::Path) -> bool {
    workspace_dir
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|n| n == "e2e-opportunities-workspace" || n.starts_with("e2e-opportunities-"))
}

/// Test-only (acceptance fixture serve): wipe posting tables between browser runs.
pub async fn acceptance_reset_postings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !state.acceptance_testing {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not available"})),
        )
            .into_response();
    }
    if !acceptance_e2e_workspace(&state.workspace_dir) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "acceptance reset blocked for non-e2e workspace"})),
        )
            .into_response();
    }
    match Store::open(&state.db_path).and_then(|s| s.clear_all_postings_for_acceptance()) {
        Ok(()) => {
            if let Some(f) = &state.acceptance_fixture {
                f.reset_sequences();
            }
            Json(json!({"ok": true})).into_response()
        }
        Err(e) => anyhow_response(e).into_response(),
    }
}

pub async fn list_postings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !state.db_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "catalog database missing"})),
        )
            .into_response();
    }
    match Store::open(&state.db_path).and_then(|s| s.list_postings()) {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => anyhow_response(e).into_response(),
    }
}

pub async fn get_posting(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !state.db_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "catalog database missing"})),
        )
            .into_response();
    }
    match Store::open(&state.db_path).and_then(|s| s.get_posting(&id)) {
        Ok(Some(p)) => Json(p).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error":"not found"}))).into_response(),
        Err(e) => anyhow_response(e).into_response(),
    }
}

pub async fn get_snapshot_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !state.db_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "catalog database missing"})),
        )
            .into_response();
    }
    match Store::open(&state.db_path).and_then(|s| {
        let kind = s.get_snapshot_retrieval_kind(&id)?;
        let raw = s.get_snapshot_html(&id)?;
        Ok((kind, raw))
    }) {
        Ok((Some(kind), Some(body))) if kind == domain::RETRIEVAL_KIND_GREENHOUSE => {
            let display = domain::snapshot_body_for_view(&body);
            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; charset=utf-8",
                )],
                display,
            )
                .into_response()
        }
        Ok((_, Some(html))) => (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            html,
        )
            .into_response(),
        Ok((_, None)) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => anyhow_response(e).into_response(),
    }
}

pub async fn patch_posting(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePostingInterpretationRequest>,
) -> impl IntoResponse {
    if !state.db_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "catalog database missing"})),
        )
            .into_response();
    }
    match Store::open(&state.db_path).and_then(|s| s.update_posting_interpretation(&id, body)) {
        Ok(p) => Json(p).into_response(),
        Err(e) if e.to_string().contains("stale_interpretation") => (
            StatusCode::CONFLICT,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => anyhow_response(e).into_response(),
    }
}

#[cfg(test)]
mod ingest_tests {
    use super::*;
    use crate::posting_http::{MockPostingHttpFetcher, PostingHttpResult};
    use std::path::PathBuf;
    use tempfile::TempDir;

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn job_html(title: &str, company: &str) -> String {
        format!(
            r#"<!DOCTYPE html><html><head><script type="application/ld+json">
{{"@context":"https://schema.org/","@type":"JobPosting","title":"{title}","hiringOrganization":{{"@type":"Organization","name":"{company}"}},"description":"Body text for the role."}}
</script></head><body></body></html>"#
        )
    }

    fn mock_state(db_path: PathBuf, url: &str, html: String) -> Arc<AppState> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fetcher = Arc::new(MockPostingHttpFetcher::single(
            url,
            PostingHttpResult {
                final_url: url.to_string(),
                status: 200,
                content_type: "text/html".to_string(),
                body: html.into_bytes(),
                redirect_chain: vec![url.to_string()],
                acceptance_transport_failure: false,
            },
        ));
        Arc::new(AppState::with_posting_fetcher(
            root.clone(),
            root.join("settings/jobdesk.yaml"),
            root.join("target/test-opp-preview"),
            db_path,
            root.join("target/test-opp-workspace"),
            fetcher,
        ))
    }

    fn api_router(state: Arc<AppState>) -> axum::Router {
        axum::Router::new()
            .route(
                "/api/opportunities/ingest",
                axum::routing::post(ingest_posting),
            )
            .route("/api/opportunities/{id}", axum::routing::get(get_posting))
            .route(
                "/api/opportunities/{id}/interpretation",
                axum::routing::patch(patch_posting),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn ingest_via_mock_fetcher_persists_posting() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let _store = Store::open(&db).unwrap();
        let url = "https://fixture.example/jobs/1";
        let state = mock_state(db.clone(), url, job_html("Role A", "Co A"));
        let app = api_router(state);
        let req = Request::builder()
            .method("POST")
            .uri("/api/opportunities/ingest")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"url":"{url}"}}"#)))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let store = Store::open(&db).unwrap();
        let list = store.list_postings().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].display_title.as_deref(), Some("Role A"));
    }

    #[tokio::test]
    async fn refetch_does_not_copy_user_overrides() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let url = "https://fixture.example/jobs/2";
        let html1 = job_html("Role V1", "Co");
        async fn post_ingest(db: std::path::PathBuf, url: &str, html: String) {
            let fetcher = Arc::new(MockPostingHttpFetcher::single(
                url,
                PostingHttpResult {
                    final_url: url.to_string(),
                    status: 200,
                    content_type: "text/html".to_string(),
                    body: html.into_bytes(),
                    redirect_chain: vec![url.to_string()],
                    acceptance_transport_failure: false,
                },
            ));
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            let st = Arc::new(AppState::with_posting_fetcher(
                root.clone(),
                root.join("settings/jobdesk.yaml"),
                root.join("target/test-opp-preview"),
                db,
                root.join("target/test-opp-workspace"),
                fetcher,
            ));
            let app = api_router(st);
            let req = Request::builder()
                .method("POST")
                .uri("/api/opportunities/ingest")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"url":"{url}"}}"#)))
                .unwrap();
            app.oneshot(req).await.unwrap();
        }
        post_ingest(db.clone(), url, html1).await;
        let detail = store
            .get_posting(&store.list_postings().unwrap()[0].id)
            .unwrap()
            .unwrap();
        let pid = detail.posting.id.clone();
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: None,
                    user_overrides: Some(json!({"title": "User Title"})),
                    mark_reviewed: Some(true),
                },
            )
            .unwrap();
        post_ingest(db.clone(), url, job_html("Role V2", "Co")).await;
        let detail2 = store.get_posting(&pid).unwrap().unwrap();
        assert_eq!(detail2.snapshots.len(), 2);
        let cur = detail2.current.as_ref().unwrap();
        assert_eq!(
            cur.merged.title.as_deref(),
            Some("Role V2"),
            "machine extract should win when user_json reset on refetch"
        );
        assert!(detail2.prior_reviewed.is_some());
    }

    #[tokio::test]
    async fn get_detail_after_refetch_json_includes_prior_reviewed() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let _store = Store::open(&db).unwrap();
        let url = "https://fixture.example/jobs/prior";
        let state = mock_state(db.clone(), url, job_html("Role A", "Co A"));
        let app = api_router(state);

        let ingest = |html: String| async {
            let st = mock_state(db.clone(), url, html);
            let app = api_router(st);
            let req = Request::builder()
                .method("POST")
                .uri("/api/opportunities/ingest")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"url":"{url}"}}"#)))
                .unwrap();
            app.oneshot(req).await.unwrap()
        };

        let res1 = ingest(job_html("Role A", "Co A")).await;
        assert_eq!(res1.status(), StatusCode::CREATED);
        let store = Store::open(&db).unwrap();
        let pid = store.list_postings().unwrap()[0].id.clone();
        let patch_state = mock_state(db.clone(), url, job_html("Role A", "Co A"));
        let patch_app = api_router(patch_state);
        let patch_req = Request::builder()
            .method("PATCH")
            .uri(format!("/api/opportunities/{pid}/interpretation"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"user_overrides":{"title":"User"},"mark_reviewed":true}"#,
            ))
            .unwrap();
        let patch_res = patch_app.oneshot(patch_req).await.unwrap();
        assert_eq!(patch_res.status(), StatusCode::OK);

        let res2 = ingest(job_html("Role B", "Co A")).await;
        assert_eq!(res2.status(), StatusCode::CREATED);

        let get_state = mock_state(db.clone(), url, job_html("Role B", "Co A"));
        let get_app = api_router(get_state);
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/api/opportunities/{pid}"))
            .body(Body::empty())
            .unwrap();
        let get_res = get_app.oneshot(get_req).await.unwrap();
        assert_eq!(get_res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(get_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert!(
            v.get("prior_reviewed").is_some(),
            "GET detail must surface prior_reviewed after refetch"
        );
        assert!(v["current"]["reviewed_at"].is_null());
    }

    #[tokio::test]
    async fn greenhouse_api_ingest_recognized() {
        use domain::greenhouse_api_job_url;
        use std::collections::HashMap;
        use std::fs;

        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let _store = Store::open(&db).unwrap();
        let ingest_url = "https://job-boards.greenhouse.io/flexport/jobs/8009198";
        let api_url = greenhouse_api_job_url("flexport", "8009198");
        let json_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../domain/tests/fixtures/greenhouse/flexport-8009198.json");
        let body = fs::read(&json_path).unwrap();
        let mut map = HashMap::new();
        map.insert(
            api_url.clone(),
            PostingHttpResult {
                final_url: api_url,
                status: 200,
                content_type: "application/json".to_string(),
                body,
                redirect_chain: vec![ingest_url.to_string()],
                acceptance_transport_failure: false,
            },
        );
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fetcher = Arc::new(MockPostingHttpFetcher::map(map));
        let state = Arc::new(AppState::with_posting_fetcher(
            root.clone(),
            root.join("settings/jobdesk.yaml"),
            root.join("target/test-gh-workspace"),
            db.clone(),
            root.join("target/test-gh-workspace-data"),
            fetcher,
        ));
        let app = api_router(state);
        let req = Request::builder()
            .method("POST")
            .uri("/api/opportunities/ingest")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"url":"{ingest_url}"}}"#)))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let store = Store::open(&db).unwrap();
        let list = store.list_postings().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].display_title.as_deref(), Some("Account Executive"));
        let detail = store.get_posting(&list[0].id).unwrap().unwrap();
        assert!(detail.latest_fetch_attempt.is_some());
        assert_eq!(
            detail
                .current
                .as_ref()
                .and_then(|c| c.extraction_outcome.as_deref()),
            Some(domain::OUTCOME_RECOGNIZED)
        );
    }

    #[tokio::test]
    async fn greenhouse_empty_body_503_records_extraction_failed() {
        use domain::greenhouse_api_job_url;
        use std::collections::HashMap;

        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let _store = Store::open(&db).unwrap();
        let ingest_url = "https://job-boards.greenhouse.io/flexport/jobs/8009198";
        let api_url = greenhouse_api_job_url("flexport", "8009198");
        let mut map = HashMap::new();
        map.insert(
            api_url.clone(),
            PostingHttpResult {
                final_url: api_url,
                status: 503,
                content_type: "application/json".to_string(),
                body: Vec::new(),
                redirect_chain: vec![ingest_url.to_string()],
                acceptance_transport_failure: false,
            },
        );
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fetcher = Arc::new(MockPostingHttpFetcher::map(map));
        let state = Arc::new(AppState::with_posting_fetcher(
            root.clone(),
            root.join("settings/jobdesk.yaml"),
            root.join("target/test-gh-503"),
            db.clone(),
            root.join("target/test-gh-503-data"),
            fetcher,
        ));
        let app = api_router(state);
        let req = Request::builder()
            .method("POST")
            .uri("/api/opportunities/ingest")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"url":"{ingest_url}"}}"#)))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let store = Store::open(&db).unwrap();
        let list = store.list_postings().unwrap();
        let detail = store.get_posting(&list[0].id).unwrap().unwrap();
        let attempt = detail.latest_fetch_attempt.unwrap();
        assert_eq!(attempt.http_status, 503);
        assert_eq!(attempt.outcome, domain::OUTCOME_EXTRACTION_FAILED);
    }

    #[tokio::test]
    async fn patch_stale_interpretation_returns_409() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let _store = Store::open(&db).unwrap();
        let url = "https://example.com/job/a";
        let html1 = job_html("T1", "Co");
        let state = mock_state(db.clone(), url, html1);
        let app = api_router(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/api/opportunities/ingest")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"url":"{url}"}}"#)))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let store = Store::open(&db).unwrap();
        let pid = store.list_postings().unwrap()[0].id.clone();
        let stale_id = store.get_posting(&pid).unwrap().unwrap().current.unwrap().id;
        let html2 = job_html("T2", "Co");
        let state2 = mock_state(db.clone(), url, html2);
        let app2 = api_router(state2);
        let req2 = Request::builder()
            .method("POST")
            .uri("/api/opportunities/ingest")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"url":"{url}"}}"#)))
            .unwrap();
        let _ = app2.clone().oneshot(req2).await.unwrap();
        let patch_req = Request::builder()
            .method("PATCH")
            .uri(format!("/api/opportunities/{pid}/interpretation"))
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"interpretation_id":"{stale_id}","user_overrides":{{"title":"nope"}}}}"#
            )))
            .unwrap();
        let patch_res = app2.oneshot(patch_req).await.unwrap();
        assert_eq!(patch_res.status(), StatusCode::CONFLICT);
    }
}

#[cfg(test)]
mod acceptance_safety_tests {
    use super::*;
    use crate::posting_http::SecurePostingHttpFetcher;
    use crate::AppState;
    use axum::body::Body;
    use axum::http::Request;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower::ServiceExt;

    #[test]
    fn acceptance_e2e_workspace_marker() {
        assert!(acceptance_e2e_workspace(Path::new(
            "/tmp/target/e2e-opportunities-workspace"
        )));
        assert!(acceptance_e2e_workspace(Path::new(
            "/home/u/proj/e2e-opportunities-isolated"
        )));
        assert!(!acceptance_e2e_workspace(Path::new(
            "/home/u/proj/workspace"
        )));
    }

    fn state_in_workspace(
        db: PathBuf,
        workspace: PathBuf,
        acceptance_testing: bool,
    ) -> Arc<AppState> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        Arc::new(AppState::with_posting_fetcher_and_acceptance(
            root.clone(),
            root.join("settings/jobdesk.yaml"),
            root.join("target/test-preview"),
            db,
            workspace,
            Arc::new(SecurePostingHttpFetcher::default()),
            acceptance_testing,
            None,
        ))
    }

    #[tokio::test]
    async fn reset_handler_not_found_when_acceptance_disabled() {
        let dir = TempDir::new().unwrap();
        let ws = dir.path().join("e2e-opportunities-workspace");
        std::fs::create_dir_all(&ws).unwrap();
        let state = state_in_workspace(ws.join("jobdesk.db"), ws, false);
        let resp = acceptance_reset_postings(axum::extract::State(state))
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn reset_handler_forbidden_on_production_workspace_dir() {
        let dir = TempDir::new().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        let state = state_in_workspace(ws.join("jobdesk.db"), ws, true);
        let resp = acceptance_reset_postings(axum::extract::State(state))
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
