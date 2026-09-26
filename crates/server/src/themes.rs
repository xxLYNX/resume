use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use theme::{catalog_fields_for_kind, load_theme_info};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CatalogFieldsQuery {
    pub kind: String,
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
    match load_theme_info(&state.root, "classic") {
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
