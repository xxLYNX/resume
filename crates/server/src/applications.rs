use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use domain::{
    APPLICATION_PHASES, DRAFT_WORKFLOW, SUBMITTED_WORKFLOWS,
};
use resolve::{default_composition, resolve_composition};
use serde::Serialize;
use store::{
    CreateApplicationRequest, ListApplicationsQuery, QuickfillRequest, Store,
    UpdateApplicationRequest, DEFAULT_THEME_ID,
};

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
    with_store(state, move |store| {
        if let Some(ref composition) = body.draft_composition {
            resolve_composition(store, composition)?;
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

#[derive(Debug, serde::Deserialize)]
pub struct SimilarQuery {
    pub limit: Option<usize>,
}

pub async fn quickfill_application(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<QuickfillRequest>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        if let Some(ref composition) = store
            .get_application(&body.from_application_id)?
            .map(|a| a.draft_composition)
        {
            resolve_composition(store, &composition)?;
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
    with_store(state, move |store| {
        let app = store
            .get_application(&app_id)?
            .ok_or_else(|| anyhow::anyhow!("application not found"))?;
        let resolved_map = resolve_composition(store, &app.draft_composition)?;
        let mut resolved = BTreeMap::new();
        for (k, v) in resolved_map {
            resolved.insert(k, v);
        }
        let sub = store.submit_application(
            &app_id,
            &workspace,
            DEFAULT_THEME_ID,
            None,
            &app.draft_composition,
            resolved,
        )?;
        Ok((StatusCode::CREATED, Json(sub)).into_response())
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
