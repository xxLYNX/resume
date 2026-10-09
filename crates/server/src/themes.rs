use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use store::DEFAULT_THEME_ID;
use theme::{catalog_fields_for_kind, list_theme_ids, load_theme_info};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CatalogFieldsQuery {
    pub kind: String,
    pub theme: Option<String>,
}

pub async fn list_themes(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match list_theme_ids(&state.root) {
        Ok(ids) => (StatusCode::OK, Json(ids)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{err:#}") })),
        )
            .into_response(),
    }
}

pub async fn get_theme(
    State(state): State<Arc<AppState>>,
    Path(theme_id): Path<String>,
) -> impl IntoResponse {
    match load_theme_info(&state.root, &theme_id) {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("{err:#}") })),
        )
            .into_response(),
    }
}

pub async fn catalog_fields(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CatalogFieldsQuery>,
) -> impl IntoResponse {
    let theme_id = query.theme.as_deref().unwrap_or(DEFAULT_THEME_ID);
    match load_theme_info(&state.root, theme_id) {
        Ok(theme) => {
            let fields = catalog_fields_for_kind(&query.kind, &theme);
            (StatusCode::OK, Json(fields)).into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{err:#}") })),
        )
            .into_response(),
    }
}
