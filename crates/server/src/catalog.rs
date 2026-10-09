use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use store::Store;

use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    pub kind: Option<String>,
}

pub async fn create_element(
    State(state): State<Arc<AppState>>,
    Json(body): Json<store::CreateElementRequest>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        if store::kind_from_str(&body.kind).is_none() {
            anyhow::bail!("unknown kind `{}`", body.kind);
        }
        let detail = store.create_element(&body)?;
        Ok((StatusCode::CREATED, Json(detail)).into_response())
    })
    .await
}

pub async fn list_elements(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let kind_filter = query.kind.clone();
    with_store(state, move |store| {
        if let Some(ref k) = kind_filter {
            if store::kind_from_str(k).is_none() {
                anyhow::bail!("unknown kind `{k}`");
            }
        }
        let kind = kind_filter.as_deref();
        Ok(Json(store.list_elements(kind)?))
    })
    .await
}

pub async fn element_usage(
    State(state): State<Arc<AppState>>,
    Path(element_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        let report = store.element_usage(&element_id)?;
        Ok(Json(report).into_response())
    })
    .await
}

pub async fn delete_element(
    State(state): State<Arc<AppState>>,
    Path(element_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        store.delete_element_if_unused(&element_id)?;
        Ok(StatusCode::NO_CONTENT.into_response())
    })
    .await
}

pub async fn get_element(
    State(state): State<Arc<AppState>>,
    Path(element_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        match store.get_element(&element_id)? {
            Some(detail) => Ok((StatusCode::OK, Json(detail)).into_response()),
            None => Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "element not found" })),
            )
                .into_response()),
        }
    })
    .await
}

pub async fn get_revision(
    State(state): State<Arc<AppState>>,
    Path(revision_id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        match store.get_revision(&revision_id)? {
            Some(rev) => Ok((StatusCode::OK, Json(rev)).into_response()),
            None => Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "revision not found" })),
            )
                .into_response()),
        }
    })
    .await
}

pub async fn create_revision(
    State(state): State<Arc<AppState>>,
    Path(element_id): Path<String>,
    Json(body): Json<store::CreateRevisionRequest>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        let rev = store.create_revision(&element_id, body.payload)?;
        Ok((StatusCode::CREATED, Json(rev)).into_response())
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
