use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use domain::{ElementSkillLink, SkillNode};
use serde::Deserialize;
use store::Store;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListSkillNodesQuery {
    pub kind: Option<String>,
    pub ids: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchSkillNodesQuery {
    pub q: String,
    pub kind: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SuggestionsQuery {
    pub tool_ids: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RelationshipsQuery {
    pub skill_id: Option<String>,
    pub element_id: Option<String>,
    pub skill_kind: Option<String>,
    pub element_kind: Option<String>,
}

pub async fn list_skill_nodes(
    State(state): State<std::sync::Arc<AppState>>,
    Query(q): Query<ListSkillNodesQuery>,
) -> Result<Json<Vec<SkillNode>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    if let Some(ids_raw) = q.ids.as_deref().filter(|s| !s.trim().is_empty()) {
        let ids: Vec<String> = ids_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let nodes = store.get_skill_nodes_by_ids(&ids).map_err(internal_err)?;
        return Ok(Json(nodes));
    }
    let nodes = store
        .list_skill_nodes(q.kind.as_deref())
        .map_err(internal_err)?;
    Ok(Json(nodes))
}

pub async fn search_skill_nodes(
    State(state): State<std::sync::Arc<AppState>>,
    Query(q): Query<SearchSkillNodesQuery>,
) -> Result<Json<Vec<SkillNode>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let limit = q.limit.unwrap_or(25).min(50);
    let nodes = store
        .search_skill_nodes(&q.q, q.kind.as_deref(), limit)
        .map_err(internal_err)?;
    Ok(Json(nodes))
}

pub async fn skill_catalog_stats(
    State(state): State<std::sync::Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let stats = store.skill_catalog_stats().map_err(internal_err)?;
    Ok(Json(serde_json::json!({ "by_kind": stats })))
}

pub async fn upsert_skill_node(
    State(state): State<std::sync::Arc<AppState>>,
    Json(body): Json<SkillNode>,
) -> Result<Json<SkillNode>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    store.upsert_skill_node(&body).map_err(bad_request)?;
    Ok(Json(body))
}

pub async fn get_element_skill_links(
    State(state): State<std::sync::Arc<AppState>>,
    Path(element_id): Path<String>,
) -> Result<Json<Vec<store::SkillLinkRow>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let rows = store
        .get_element_skill_links(&element_id)
        .map_err(internal_err)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct PutElementSkillLinksBody {
    pub links: Vec<ElementSkillLink>,
}

pub async fn put_element_skill_links(
    State(state): State<std::sync::Arc<AppState>>,
    Path(element_id): Path<String>,
    Json(body): Json<PutElementSkillLinksBody>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    store
        .set_element_skill_links(&element_id, &body.links)
        .map_err(bad_request)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SkillLinkSummariesQuery {
    pub ids: String,
}

pub async fn element_skill_link_summaries(
    State(state): State<std::sync::Arc<AppState>>,
    Query(q): Query<SkillLinkSummariesQuery>,
) -> Result<Json<Vec<store::ElementSkillLinkSummary>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let element_ids: Vec<String> = q
        .ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let rows = store
        .element_skill_link_summaries(&element_ids)
        .map_err(internal_err)?;
    Ok(Json(rows))
}

pub async fn list_skill_relationships(
    State(state): State<std::sync::Arc<AppState>>,
    Query(q): Query<RelationshipsQuery>,
) -> Result<Json<Vec<store::SkillRelationshipRow>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let filter = store::ListSkillRelationshipsFilter {
        skill_id: q.skill_id,
        element_id: q.element_id,
        skill_kind: q.skill_kind,
        element_kind: q.element_kind,
    };
    let rows = store
        .list_skill_relationships(&filter)
        .map_err(internal_err)?;
    Ok(Json(rows))
}

pub async fn competency_suggestions(
    State(state): State<std::sync::Arc<AppState>>,
    Query(q): Query<SuggestionsQuery>,
) -> Result<Json<Vec<store::CompetencySuggestion>>, (StatusCode, Json<serde_json::Value>)> {
    let store = Store::open(&state.db_path).map_err(internal_err)?;
    let tool_ids: Vec<String> = q
        .tool_ids
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let rows = store
        .suggest_competencies_for_tools(&tool_ids)
        .map_err(internal_err)?;
    Ok(Json(rows))
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
