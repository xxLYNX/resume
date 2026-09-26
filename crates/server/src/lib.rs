mod applications;
mod assets;
mod catalog;
mod presets;
mod themes;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use domain::compute_snapshot_hash;
use render_pdf::PdfEngine;
use render_tex::{jobdesk_build_aligned, jobdesk_build_composition_to_dir};
use resolve::{default_composition, resolve_composition};
use serde::{Deserialize, Serialize};
use store::Store;
use theme::load_theme_info;
use tower_http::trace::TraceLayer;

#[derive(Default)]
struct PreviewCache {
    cache_key: Option<String>,
    content_hash: Option<String>,
    pdf_engine: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub preview_dir: PathBuf,
    pub db_path: PathBuf,
    pub workspace_dir: PathBuf,
    pub preview_cache: Arc<Mutex<PreviewCache>>,
}

impl AppState {
    pub fn new(
        root: PathBuf,
        config_path: PathBuf,
        preview_dir: PathBuf,
        db_path: PathBuf,
        workspace_dir: PathBuf,
    ) -> Self {
        Self {
            root,
            config_path,
            preview_dir,
            db_path,
            workspace_dir,
            preview_cache: Arc::new(Mutex::new(PreviewCache::default())),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct PreviewRequest {
    /// When set, preview uses this application's draft or canonical submission.
    pub application_id: Option<String>,
    /// `draft` (default with application_id) or `submitted` (canonical manifest).
    pub mode: Option<String>,
    /// Unsaved draft from the UI; used for live preview when `mode` is `draft`.
    pub draft_composition: Option<domain::Composition>,
}

#[derive(Serialize)]
struct PreviewResponse {
    ok: bool,
    /// Same logical resume as last preview; PDF was not recompiled.
    #[serde(default)]
    unchanged: bool,
    content_hash: Option<String>,
    pdf_engine: Option<String>,
    pdf_url: Option<String>,
    tex_path: Option<String>,
    error: Option<String>,
    log_tail: Option<String>,
    preview_source: Option<String>,
    submission_content_hash: Option<String>,
    draft_differs_from_submission: Option<bool>,
}

pub async fn run(state: AppState, host: &str, port: u16) -> Result<()> {
    let shared = Arc::new(state);
    let app = Router::new()
        .route("/", get(index))
        .route("/api/preview", post(run_preview))
        .route("/api/preview/resume.pdf", get(get_preview_pdf))
        .route("/api/health", get(health))
        .route("/api/catalog/elements", get(catalog::list_elements))
        .route("/api/catalog/elements/{id}", get(catalog::get_element))
        .route(
            "/api/catalog/elements/{id}/revisions",
            post(catalog::create_revision),
        )
        .route("/api/catalog/revisions/{id}", get(catalog::get_revision))
        .route("/api/applications/statuses", get(applications::list_statuses))
        .route("/api/applications", get(applications::list_applications))
        .route("/api/applications", post(applications::create_application))
        .route("/api/applications/{id}", get(applications::get_application))
        .route("/api/applications/{id}", patch(applications::update_application))
        .route("/api/applications/{id}", delete(applications::delete_application))
        .route(
            "/api/applications/{id}/similar",
            get(applications::list_similar),
        )
        .route(
            "/api/applications/{id}/quickfill",
            post(applications::quickfill_application),
        )
        .route(
            "/api/applications/{id}/reset-draft",
            post(applications::reset_draft),
        )
        .route(
            "/api/applications/{id}/submit",
            post(applications::submit_application),
        )
        .route(
            "/api/submissions/{id}",
            get(applications::get_submission),
        )
        .route("/api/composition/default", get(default_composition_handler))
        .route("/api/themes/{id}", get(themes::get_theme))
        .route("/api/catalog/field-schema", get(themes::catalog_fields))
        .route("/api/presets", get(presets::list_presets))
        .route("/api/presets", post(presets::create_preset))
        .route("/api/presets/{id}", get(presets::get_preset))
        .route("/api/presets/{id}", patch(presets::update_preset))
        .route("/api/presets/{id}", delete(presets::delete_preset))
        .layer(TraceLayer::new_for_http())
        .with_state(shared);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .context("parse listen address")?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;

    eprintln!("jobdesk UI: http://{addr}");
    axum::serve(listener, app).await.context("serve")?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(assets::INDEX)
}

async fn health() -> &'static str {
    "ok"
}

async fn default_composition_handler(State(state): State<Arc<AppState>>) -> Response {
    if !state.db_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "catalog database missing; run: cargo run -p jobdesk -- import"
            })),
        )
            .into_response();
    }
    match Store::open(&state.db_path).and_then(|store| default_composition(&store)) {
        Ok(comp) => (StatusCode::OK, Json(comp)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{err:#}") })),
        )
            .into_response(),
    }
}

async fn run_preview(
    State(state): State<Arc<AppState>>,
    body: Option<Json<PreviewRequest>>,
) -> Json<PreviewResponse> {
    let req = body.map(|j| j.0).unwrap_or_default();
    match build_preview(&state, &req) {
        Ok(result) => Json(PreviewResponse {
            ok: true,
            unchanged: result.unchanged,
            content_hash: Some(result.build.content_hash),
            pdf_engine: result.build.pdf_engine,
            pdf_url: Some("/api/preview/resume.pdf".into()),
            tex_path: Some(result.build.tex_path.display().to_string()),
            error: None,
            log_tail: None,
            preview_source: Some(result.preview_source),
            submission_content_hash: result.submission_content_hash,
            draft_differs_from_submission: result.draft_differs_from_submission,
        }),
        Err(err) => {
            let log_tail = read_log_tail(&state.preview_dir.join("build.log"));
            Json(PreviewResponse {
                ok: false,
                unchanged: false,
                content_hash: None,
                pdf_engine: PdfEngine::detect().map(|e| e.name().to_string()),
                pdf_url: None,
                tex_path: None,
                error: Some(format!("{err:#}")),
                log_tail,
                preview_source: None,
                submission_content_hash: None,
                draft_differs_from_submission: None,
            })
        }
    }
}

async fn get_preview_pdf(State(state): State<Arc<AppState>>) -> Response {
    let pdf_path = state.preview_dir.join("resume.pdf");
    let Ok(bytes) = std::fs::read(&pdf_path) else {
        return (StatusCode::NOT_FOUND, "PDF not built yet; POST /api/preview").into_response();
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/pdf"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );

    (StatusCode::OK, headers, bytes).into_response()
}

struct PreviewBuildMeta {
    build: render_tex::LegacyBuildResult,
    preview_source: String,
    submission_content_hash: Option<String>,
    draft_differs_from_submission: Option<bool>,
    unchanged: bool,
}

fn build_preview(state: &AppState, req: &PreviewRequest) -> Result<PreviewBuildMeta> {
    if !state.db_path.exists() {
        anyhow::bail!(
            "catalog database missing at {}; run: cargo run -p jobdesk -- import",
            state.db_path.display()
        );
    }
    let store = Store::open(&state.db_path)?;

    let application_id = req.application_id.as_deref();
    let mode = req.mode.as_deref().unwrap_or("draft");

    let submission_snapshot = if let Some(app_id) = application_id {
        store
            .get_canonical_submission_with_root(&state.workspace_dir, app_id)?
            .map(|s| s.manifest.snapshot_hash.clone())
    } else {
        None
    };

    let cache_key = preview_cache_key(state, &store, req)?;
    let pdf_path = state.preview_dir.join("resume.pdf");
    if pdf_path.is_file() {
        if let Ok(cache) = state.preview_cache.lock() {
            if cache.cache_key.as_deref() == Some(cache_key.as_str()) {
                if let (Some(content_hash), pdf_engine) =
                    (cache.content_hash.clone(), cache.pdf_engine.clone())
                {
                    let (preview_source, draft_differs) =
                        preview_meta(&store, state, req, application_id, mode, &submission_snapshot)?;
                    return Ok(PreviewBuildMeta {
                        build: render_tex::LegacyBuildResult {
                            tex: String::new(),
                            tex_path: state.preview_dir.join("resume.tex"),
                            pdf_path: Some(pdf_path),
                            content_hash,
                            pdf_engine,
                        },
                        preview_source,
                        submission_content_hash: submission_snapshot,
                        draft_differs_from_submission: draft_differs,
                        unchanged: true,
                    });
                }
            }
        }
    }

    let build = match (application_id, mode, req.draft_composition.as_ref()) {
        (Some(_), "submitted", _) | (None, _, _) | (Some(_), _, None) => jobdesk_build_aligned(
            &store,
            &state.workspace_dir,
            &state.root,
            &state.config_path,
            &state.preview_dir,
            true,
            application_id,
            mode,
        )?,
        (Some(_), _, Some(composition)) => {
            resolve_composition(&store, composition)?;
            jobdesk_build_composition_to_dir(
                &store,
                &state.root,
                &state.config_path,
                &state.preview_dir,
                composition,
                true,
            )?
        }
    };

    let (preview_source, draft_differs) =
        preview_meta(&store, state, req, application_id, mode, &submission_snapshot)?;

    if let Ok(mut cache) = state.preview_cache.lock() {
        cache.cache_key = Some(cache_key);
        cache.content_hash = Some(build.content_hash.clone());
        cache.pdf_engine = build.pdf_engine.clone();
    }

    Ok(PreviewBuildMeta {
        build,
        preview_source,
        submission_content_hash: submission_snapshot,
        draft_differs_from_submission: draft_differs,
        unchanged: false,
    })
}

fn preview_meta(
    store: &Store,
    state: &AppState,
    req: &PreviewRequest,
    application_id: Option<&str>,
    mode: &str,
    submission_snapshot: &Option<String>,
) -> Result<(String, Option<bool>)> {
    Ok(match application_id {
        Some(_) if mode == "submitted" => ("submitted".to_string(), None),
        Some(app_id) => {
            let app = store
                .get_application(app_id)?
                .ok_or_else(|| anyhow::anyhow!("application not found"))?;
            let draft_for_hash = req
                .draft_composition
                .as_ref()
                .unwrap_or(&app.draft_composition);
            let draft_snapshot = draft_snapshot_hash(&state.root, store, draft_for_hash)?;
            let differs = submission_snapshot
                .as_ref()
                .map(|h| h != &draft_snapshot)
                .unwrap_or(false);
            (
                if req.draft_composition.is_some() {
                    "application_draft_live"
                } else {
                    "application_draft"
                }
                .to_string(),
                Some(differs),
            )
        }
        None => ("catalog_default".to_string(), None),
    })
}

fn preview_cache_key(
    state: &AppState,
    store: &Store,
    req: &PreviewRequest,
) -> Result<String> {
    let application_id = req.application_id.as_deref();
    let mode = req.mode.as_deref().unwrap_or("draft");
    match (application_id, mode) {
        (None, _) => {
            let composition = default_composition(store)?;
            Ok(format!(
                "catalog:{}",
                draft_snapshot_hash(&state.root, store, &composition)?
            ))
        }
        (Some(app_id), "submitted") => {
            let sub = store
                .get_canonical_submission_with_root(&state.workspace_dir, app_id)?
                .with_context(|| format!("no canonical submission for application `{app_id}`"))?;
            Ok(format!(
                "submitted:{}:{}",
                app_id, sub.manifest.snapshot_hash
            ))
        }
        (Some(app_id), _) => {
            let composition = if let Some(c) = &req.draft_composition {
                c.clone()
            } else {
                store
                    .get_application(app_id)?
                    .ok_or_else(|| anyhow::anyhow!("application not found"))?
                    .draft_composition
            };
            Ok(format!(
                "draft:{}:{}",
                app_id,
                draft_snapshot_hash(&state.root, store, &composition)?
            ))
        }
    }
}

fn draft_snapshot_hash(
    project_root: &std::path::Path,
    store: &Store,
    composition: &domain::Composition,
) -> Result<String> {
    let resolved_map = resolve_composition(store, composition)?;
    let mut resolved = std::collections::BTreeMap::new();
    for (k, v) in resolved_map {
        resolved.insert(k, v);
    }
    let theme_hash = load_theme_info(project_root, store::DEFAULT_THEME_ID)
        .ok()
        .map(|t| t.files_hash);
    Ok(compute_snapshot_hash(
        store::DEFAULT_THEME_ID,
        theme_hash.as_deref(),
        composition,
        &resolved,
    ))
}

fn read_log_tail(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(40);
    Some(lines[start..].join("\n"))
}
