use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use store::{CreatePresetRequest, Store, UpdatePresetRequest, DEFAULT_THEME_ID};

use crate::resolve_checks::resolve_composition_checked;
use crate::AppState;

pub async fn list_presets(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    with_store(state, |store| Ok(Json(store.list_presets()?))).await
}

pub async fn create_preset(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreatePresetRequest>,
) -> impl IntoResponse {
    let project_root = state.root.clone();
    with_store(state, move |store| {
        resolve_composition_checked(store, &project_root, DEFAULT_THEME_ID, &body.composition)?;
        let preset = store.create_preset(body)?;
        Ok((StatusCode::CREATED, Json(preset)).into_response())
    })
    .await
}

pub async fn get_preset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        match store.get_preset(&id)? {
            Some(p) => Ok((StatusCode::OK, Json(p)).into_response()),
            None => Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "preset not found" })),
            )
                .into_response()),
        }
    })
    .await
}

pub async fn update_preset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePresetRequest>,
) -> impl IntoResponse {
    let project_root = state.root.clone();
    with_store(state, move |store| {
        if let Some(ref composition) = body.composition {
            resolve_composition_checked(store, &project_root, DEFAULT_THEME_ID, composition)?;
        }
        let preset = store.update_preset(&id, body)?;
        Ok(Json(preset).into_response())
    })
    .await
}

pub async fn delete_preset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        if store.delete_preset(&id)? {
            Ok(StatusCode::NO_CONTENT.into_response())
        } else {
            Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "preset not found" })),
            )
                .into_response())
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
