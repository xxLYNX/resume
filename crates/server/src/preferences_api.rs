use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use theme::preferences::{load_preferences, set_default_theme, DefaultThemeSlot};

use crate::AppState;

pub async fn get_preferences(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match load_preferences(&state.root, &state.workspace_dir) {
        Ok(prefs) => Json(prefs).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{err:#}") })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct SetDefaultThemeRequest {
    pub slot: String,
    pub theme_id: String,
}

pub async fn put_default_theme(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetDefaultThemeRequest>,
) -> impl IntoResponse {
    let Some(slot) = parse_slot(&body.slot) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "slot must be catalog_preview, showcase, or application"
            })),
        )
            .into_response();
    };
    match set_default_theme(
        &state.root,
        &state.workspace_dir,
        slot,
        body.theme_id.trim(),
    ) {
        Ok(prefs) => Json(prefs).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{err:#}") })),
        )
            .into_response(),
    }
}

fn parse_slot(slot: &str) -> Option<DefaultThemeSlot> {
    match slot {
        "catalog_preview" => Some(DefaultThemeSlot::CatalogPreview),
        "showcase" => Some(DefaultThemeSlot::Showcase),
        "application" => Some(DefaultThemeSlot::Application),
        _ => None,
    }
}
