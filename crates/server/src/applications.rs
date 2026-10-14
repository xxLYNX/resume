use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use domain::{
    Composition, APPLICATION_PHASES, DRAFT_WORKFLOW, SUBMITTED_WORKFLOWS,
};
use resolve::default_composition;

use crate::resolve_checks::resolve_composition_checked;
use serde::{Deserialize, Serialize};
use store::{
    default_ledger_download_path, CreateApplicationRequest, ListApplicationsQuery,
    QuickfillRequest, Store, UpdateApplicationRequest, DEFAULT_THEME_ID,
};

use theme::ensure_theme_snapshot;

use crate::AppState;

#[derive(Serialize)]
pub struct ApplicationLifecycleMeta {
    pub phases: &'static [&'static str],
    pub draft_workflow: &'static str,
    pub submitted_workflows: &'static [&'static str],
}

pub async fn list_statuses() -> Json<ApplicationLifecycleMeta> {
    Json(ApplicationLifecycleMeta {
        phases: APPLICATION_PHASES,
        draft_workflow: DRAFT_WORKFLOW,
        submitted_workflows: SUBMITTED_WORKFLOWS,
    })
}

pub async fn list_applications(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<ListApplicationsQuery>,
) -> impl IntoResponse {
    with_store(state, move |store| Ok(Json(store.list_applications(&filter)?))).await
}

pub async fn create_application(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateApplicationRequest>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        let draft = if let Some(ref clone_id) = body.clone_from_application_id {
            store
                .get_application(clone_id)?
                .map(|a| a.draft_composition)
                .ok_or_else(|| anyhow::anyhow!("clone_from application not found"))?
        } else if let Some(ref preset_id) = body.clone_from_preset_id {
            store
                .get_preset(preset_id)?
                .map(|p| p.composition)
                .ok_or_else(|| anyhow::anyhow!("clone_from preset not found"))?
        } else {
            default_composition(store)?
        };
        let app = store.create_application(body, draft)?;
        Ok((StatusCode::CREATED, Json(app)).into_response())
    })
    .await
}

pub async fn get_application(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        match store.get_application(&app_id)? {
            Some(app) => Ok((StatusCode::OK, Json(app)).into_response()),
            None => Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "application not found" })),
            )
                .into_response()),
        }
    })
    .await
}

pub async fn update_application(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<UpdateApplicationRequest>,
) -> impl IntoResponse {
    let project_root = state.root.clone();
    with_store(state, move |store| {
        if let Some(ref composition) = body.draft_composition {
            resolve_composition_checked(store, &project_root, DEFAULT_THEME_ID, composition)?;
        }
        let app = store.update_application(&app_id, body)?;
        Ok(Json(app).into_response())
    })
    .await
}

pub async fn delete_application(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        if store.delete_application(&app_id)? {
            Ok(StatusCode::NO_CONTENT.into_response())
        } else {
            Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "application not found" })),
            )
                .into_response())
        }
    })
    .await
}

pub async fn list_similar(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<SimilarQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(5).clamp(1, 20);
    with_store(state, move |store| {
        let matches = store.rank_similar_applications(&app_id, limit)?;
        Ok(Json(matches).into_response())
    })
    .await
}

#[derive(Serialize)]
pub struct PostingHintsResponse {
    pub hints: Vec<domain::PostingHintRow>,
    /// True when hints used pasted posting text; false when only title/tags fallback.
    pub used_posting_text: bool,
}

pub async fn posting_hints(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        let app = store
            .get_application(&app_id)?
            .ok_or_else(|| anyhow::anyhow!("application not found"))?;
        let used_posting_text = !app.posting_text.trim().is_empty();
        let hints = store.application_posting_hints(&app_id)?;
        Ok(Json(PostingHintsResponse {
            hints,
            used_posting_text,
        })
            .into_response())
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct PostingHintsPreviewBody {
    pub posting_text: String,
    #[serde(default)]
    pub draft_composition: Option<Composition>,
}

/// Analyze unsaved posting text and/or draft composition (read-only).
pub async fn posting_hints_preview(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<PostingHintsPreviewBody>,
) -> impl IntoResponse {
    let project_root = state.root.clone();
    with_store(state, move |store| {
        let app = store
            .get_application(&app_id)?
            .ok_or_else(|| anyhow::anyhow!("application not found"))?;
        let composition = body
            .draft_composition
            .as_ref()
            .unwrap_or(&app.draft_composition);
        resolve_composition_checked(store, &project_root, DEFAULT_THEME_ID, composition)?;
        let posting_text = body.posting_text.trim();
        let used_posting_text = !posting_text.is_empty();
        let text = if used_posting_text {
            posting_text.to_string()
        } else {
            let mut fallback = format!("{} {}", app.title, app.company);
            for tag in &app.tags {
                fallback.push(' ');
                fallback.push_str(tag);
            }
            fallback
        };
        let hints = store.posting_skill_hints(&text, composition)?;
        Ok(Json(PostingHintsResponse {
            hints,
            used_posting_text,
        })
            .into_response())
    })
    .await
}

#[derive(Debug, serde::Deserialize)]
pub struct SimilarQuery {
    pub limit: Option<usize>,
}

pub async fn quickfill_application(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<QuickfillRequest>,
) -> impl IntoResponse {
    let project_root = state.root.clone();
    with_store(state, move |store| {
        if let Some(ref composition) = store
            .get_application(&body.from_application_id)?
            .map(|a| a.draft_composition)
        {
            resolve_composition_checked(store, &project_root, DEFAULT_THEME_ID, composition)?;
        }
        let app = store.quickfill_from_application(&app_id, &body.from_application_id)?;
        Ok(Json(app).into_response())
    })
    .await
}

pub async fn reset_draft(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        let draft = default_composition(store)?;
        let app = store.update_application(
            &app_id,
            UpdateApplicationRequest {
                draft_composition: Some(draft),
                ..Default::default()
            },
        )?;
        Ok(Json(app).into_response())
    })
    .await
}

pub async fn submit_application(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> impl IntoResponse {
    let workspace = state.workspace_dir.clone();
    let project_root = state.root.clone();
    with_store(state, move |store| {
        let app = store
            .get_application(&app_id)?
            .ok_or_else(|| anyhow::anyhow!("application not found"))?;
        let resolved_map = resolve_composition_checked(
            store,
            &project_root,
            DEFAULT_THEME_ID,
            &app.draft_composition,
        )?;
        let mut resolved = BTreeMap::new();
        for (k, v) in resolved_map {
            resolved.insert(k, v);
        }
        let snap_dir = workspace.join("artifacts/theme_snapshots");
        let theme_files_hash =
            ensure_theme_snapshot(&project_root, DEFAULT_THEME_ID, &snap_dir)?;
        let sub = store.submit_application(
            &app_id,
            &workspace,
            DEFAULT_THEME_ID,
            Some(theme_files_hash),
            &app.draft_composition,
            resolved,
        )?;
        Ok((StatusCode::CREATED, Json(sub)).into_response())
    })
    .await
}

#[derive(Deserialize)]
pub struct ExportLedgerRequest {
    pub output: Option<std::path::PathBuf>,
}

#[derive(Serialize)]
pub struct ExportLedgerResponse {
    pub path: String,
}

pub async fn export_application_ledger(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ExportLedgerRequest>,
) -> impl IntoResponse {
    let workspace = state.workspace_dir.clone();
    with_store(state, move |store| {
        let dest = body
            .output
            .map(Ok)
            .unwrap_or_else(default_ledger_download_path)?;
        let build_dir = workspace.join("artifacts/ledger-build");
        let path = store.export_application_ledger_pdf(&build_dir, &dest)?;
        Ok((
            StatusCode::OK,
            Json(ExportLedgerResponse {
                path: path.display().to_string(),
            }),
        )
            .into_response())
    })
    .await
}

pub async fn get_submission(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<String>,
) -> impl IntoResponse {
    let workspace = state.workspace_dir.clone();
    with_store(state, move |store| {
        match store.get_submission_with_root(&workspace, &sub_id)? {
            Some(sub) => Ok((StatusCode::OK, Json(sub)).into_response()),
            None => Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "submission not found" })),
            )
                .into_response()),
        }
    })
    .await
}

async fn with_store<F, T>(state: Arc<AppState>, f: F) -> impl IntoResponse
where
    F: FnOnce(&Store) -> anyhow::Result<T>,
    T: IntoResponse,
{
    if !state.db_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "catalog database missing; run: cargo run -p jobdesk -- import"
            })),
        )
            .into_response();
    }

    match Store::open(&state.db_path).and_then(|store| f(&store)) {
        Ok(response) => response.into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{err:#}") })),
        )
            .into_response(),
    }
}
