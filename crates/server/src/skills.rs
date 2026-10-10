use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use domain::{RevisionSkillLink, SkillNode};
use serde::Deserialize;
use store::Store;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListSkillNodesQuery {
    pub kind: Option<String>,
}

pub async fn list_skill_nodes(
    State(state): State<std::sync::Arc<AppState>>,
    Query(q): Query<ListSkillNodesQuery>,
) -> Result<Json<Vec<SkillNode>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let nodes = store
        .list_skill_nodes(q.kind.as_deref())
        .map_err(internal_err)?;
    Ok(Json(nodes))
}

pub async fn upsert_skill_node(
    State(state): State<std::sync::Arc<AppState>>,
    Json(body): Json<SkillNode>,
) -> Result<Json<SkillNode>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    store.upsert_skill_node(&body).map_err(bad_request)?;
    Ok(Json(body))
}

pub async fn get_revision_skill_links(
    State(state): State<std::sync::Arc<AppState>>,
    Path(revision_id): Path<String>,
) -> Result<Json<Vec<store::SkillLinkRow>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let rows = store
        .get_revision_skill_links(&revision_id)
        .map_err(internal_err)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct PutRevisionSkillLinksBody {
    pub links: Vec<RevisionSkillLink>,
}

pub async fn put_revision_skill_links(
    State(state): State<std::sync::Arc<AppState>>,
    Path(revision_id): Path<String>,
    Json(body): Json<PutRevisionSkillLinksBody>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    store
        .set_revision_skill_links(&revision_id, &body.links)
        .map_err(bad_request)?;
    Ok(StatusCode::NO_CONTENT)
}

fn internal_err(e: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": e.to_string() })),
    )
}

fn bad_request(e: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": e.to_string() })),
    )
}
