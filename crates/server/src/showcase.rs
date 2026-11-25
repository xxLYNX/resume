use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use domain::{sha256_prefixed, ShowcaseConflict};
use render_tex::approve_showcase;
use serde::Deserialize;
use store::Store;

use crate::AppState;

fn is_showcase_conflict(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| cause.is::<ShowcaseConflict>())
}

pub async fn get_showcase(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    with_store(state, |store| {
        let draft = store.get_showcase()?;
        let publications = store.list_showcase_publications()?;
        let current = match draft.current_publication_id.as_deref() {
            Some(id) => store.get_showcase_publication(id)?,
            None => None,
        };
        Ok(Json(serde_json::json!({
            "draft": draft,
            "current_publication": current,
            "publications": publications,
        }))
        .into_response())
    })
    .await
}

#[derive(Deserialize)]
pub struct SaveShowcaseDraftRequest {
    pub theme_id: String,
    pub composition: domain::Composition,
    pub updated_at: String,
}

pub async fn save_showcase_draft(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveShowcaseDraftRequest>,
) -> impl IntoResponse {
    let root = state.root.clone();
    with_store(state, move |store| {
        theme::load_theme_info(&root, body.theme_id.trim())?;
        match store.save_showcase_draft_versioned(
            body.theme_id.trim(),
            &body.composition,
            Some(body.updated_at.trim()),
        ) {
            Ok(draft) => Ok(Json(draft).into_response()),
            Err(err) if is_showcase_conflict(&err) => Ok((
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": format!("{err:#}") })),
            )
                .into_response()),
            Err(err) => Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("{err:#}") })),
            )
                .into_response()),
        }
    })
    .await
}

#[derive(Deserialize)]
pub struct ApproveShowcaseRequest {
    pub updated_at: String,
}

pub async fn approve_showcase_draft(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ApproveShowcaseRequest>,
) -> impl IntoResponse {
    let root = state.root.clone();
    let workspace = state.workspace_dir.clone();
    with_store(state, move |store| {
        let draft = store.get_showcase()?;
        if draft.updated_at != body.updated_at {
            return Ok((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "showcase draft was saved again — reload the draft and preview it before approving"
                })),
            )
                .into_response());
        }
        match approve_showcase(&store, &root, &workspace) {
            Ok(publication) => Ok((StatusCode::CREATED, Json(publication)).into_response()),
            Err(err) if is_showcase_conflict(&err) => Ok((
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": format!("{err:#}") })),
            )
                .into_response()),
            Err(err) => Err(err),
        }
    })
    .await
}

#[derive(Deserialize)]
pub struct RestoreShowcaseRequest {
    pub publication_id: String,
}

#[derive(Deserialize)]
pub struct ApplyShowcaseDefaultThemeRequest {
    pub updated_at: String,
}

pub async fn apply_showcase_default_theme(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ApplyShowcaseDefaultThemeRequest>,
) -> impl IntoResponse {
    let root = state.root.clone();
    let workspace = state.workspace_dir.clone();
    with_store(state, move |store| {
        let theme_id = theme::preferences::load_preferences(&root, &workspace)?
            .resume
            .default_themes
            .showcase;
        theme::load_theme_info(&root, &theme_id)?;
        match store.apply_showcase_theme(&theme_id, &body.updated_at) {
            Ok(draft) => Ok(Json(draft).into_response()),
            Err(err) if is_showcase_conflict(&err) => Ok((
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": format!("{err:#}") })),
            )
                .into_response()),
            Err(err) => Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("{err:#}") })),
            )
                .into_response()),
        }
    })
    .await
}

pub async fn restore_showcase(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RestoreShowcaseRequest>,
) -> impl IntoResponse {
    let workspace = state.workspace_dir.clone();
    with_store(state, move |store| {
        let draft = store.restore_showcase_pointer(&workspace, &body.publication_id)?;
        Ok(Json(draft).into_response())
    })
    .await
}

pub async fn get_showcase_publication(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    with_store(state, move |store| {
        match store.get_showcase_publication(&id)? {
            Some(publication) => Ok(Json(publication).into_response()),
            None => Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "unknown showcase publication" })),
            )
                .into_response()),
        }
    })
    .await
}

pub async fn get_showcase_publication_pdf(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let workspace = state.workspace_dir.clone();
    with_store(state, move |store| {
        let Some(publication) = store.get_showcase_publication(&id)? else {
            return Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "unknown showcase publication" })),
            )
                .into_response());
        };
        let path = workspace.join(&publication.pdf_relpath);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                return Ok((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "approved PDF is missing — the publication was not served"
                    })),
                )
                    .into_response());
            }
        };
        if sha256_prefixed(&bytes) != publication.pdf_sha256 {
            return Ok((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "approved PDF checksum does not match — the publication was not served"
                })),
            )
                .into_response());
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/pdf"),
        );
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
        headers.insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_static("inline; filename=\"showcase-resume.pdf\""),
        );
        Ok((StatusCode::OK, headers, bytes).into_response())
    })
    .await
}

#[cfg(test)]
mod j2_api_tests {
    use super::*;
    use crate::preferences_api;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use domain::{empty_showcase_composition, Composition};
    use serde_json::json;
    use std::path::PathBuf;
    use domain::{CompositionItem, CompositionSection, sha256_prefixed};
    use store::catalog::CreateElementRequest;
    use store::{CreateApplicationRequest, Store};
    use tempfile::TempDir;
    use theme::preferences::{set_default_theme, DefaultThemeSlot};
    use tower::ServiceExt;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn header_showcase_composition(element_id: &str, revision_id: &str) -> domain::Composition {
        domain::Composition {
            sections: vec![CompositionSection {
                section_key: "header".into(),
                items: vec![CompositionItem {
                    element_id: element_id.into(),
                    revision_id: revision_id.into(),
                    resume_tag_skill_ids: vec![],
                }],
            }],
        }
    }

    async fn post_json(
        app: &axum::Router,
        method: &str,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(json!({ "raw": String::from_utf8_lossy(&bytes) }));
        (status, json)
    }

    fn mock_state(db_path: PathBuf, workspace: PathBuf) -> Arc<AppState> {
        let root = root();
        Arc::new(AppState::new(
            root.clone(),
            root.join("settings/jobdesk.yaml"),
            workspace.join("preview"),
            db_path,
            workspace,
        ))
    }

    fn showcase_router(state: Arc<AppState>) -> axum::Router {
        axum::Router::new()
            .route("/api/showcase", axum::routing::get(get_showcase))
            .route("/api/showcase/draft", axum::routing::post(save_showcase_draft))
            .route(
                "/api/showcase/apply-default-theme",
                axum::routing::post(apply_showcase_default_theme),
            )
            .route(
                "/api/showcase/approve",
                axum::routing::post(approve_showcase_draft),
            )
            .route("/api/showcase/restore", axum::routing::post(restore_showcase))
            .route(
                "/api/showcase/publications/{id}/resume.pdf",
                axum::routing::get(get_showcase_publication_pdf),
            )
            .route("/api/preferences", axum::routing::get(preferences_api::get_preferences))
            .route(
                "/api/preferences/default-theme",
                axum::routing::post(preferences_api::put_default_theme),
            )
            .with_state(state)
    }

    fn applications_router(state: Arc<AppState>) -> axum::Router {
        axum::Router::new()
            .route("/api/applications", axum::routing::post(crate::applications::create_application))
            .route(
                "/api/applications/{id}",
                axum::routing::patch(crate::applications::update_application),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn apply_configured_showcase_default_changes_saved_theme_only() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let workspace = dir.path().to_path_buf();
        let root = root();
        set_default_theme(&root, &workspace, DefaultThemeSlot::Showcase, "ats-classic").unwrap();
        let store = Store::open(&db).unwrap();
        let state = mock_state(db.clone(), workspace.clone());
        let app = showcase_router(state);
        let updated_at = store.get_showcase().unwrap().updated_at;
        let draft = empty_showcase_composition();
        let (status, saved) = post_json(
            &app,
            "POST",
            "/api/showcase/draft",
            json!({
                "theme_id": "classic",
                "composition": draft,
                "updated_at": updated_at,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let updated_at = saved["updated_at"].as_str().unwrap().to_string();

        let apply = Request::builder()
            .method("POST")
            .uri("/api/showcase/apply-default-theme")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "updated_at": updated_at })).unwrap(),
            ))
            .unwrap();
        let apply_resp = app.clone().oneshot(apply).await.unwrap();
        assert_eq!(apply_resp.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&axum::body::to_bytes(apply_resp.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(body["theme_id"], "ats-classic");
    }

    #[tokio::test]
    async fn stale_showcase_approve_and_apply_default_return_conflict() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let workspace = dir.path().to_path_buf();
        let store = Store::open(&db).unwrap();
        let draft = store.save_showcase_draft("classic", &empty_showcase_composition()).unwrap();
        let stale = draft.updated_at.clone();
        store
            .save_showcase_draft("classic", &empty_showcase_composition())
            .unwrap();
        let state = mock_state(db, workspace);
        let app = showcase_router(state);

        for uri in ["/api/showcase/approve", "/api/showcase/apply-default-theme"] {
            let req = Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({ "updated_at": stale })).unwrap(),
                ))
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::CONFLICT, "uri={uri}");
        }
    }

    #[tokio::test]
    async fn invalid_showcase_draft_theme_is_rejected() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let workspace = dir.path().to_path_buf();
        let store = Store::open(&db).unwrap();
        let updated_at = store.get_showcase().unwrap().updated_at;
        let state = mock_state(db.clone(), workspace);
        let app = showcase_router(state);
        let (status, _) = post_json(
            &app,
            "POST",
            "/api/showcase/draft",
            json!({
                "theme_id": "no-such-theme",
                "composition": empty_showcase_composition(),
                "updated_at": updated_at,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn stale_draft_save_returns_conflict() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let workspace = dir.path().to_path_buf();
        let store = Store::open(&db).unwrap();
        let stale = store.get_showcase().unwrap().updated_at;
        store
            .save_showcase_draft("classic", &empty_showcase_composition())
            .unwrap();
        let state = mock_state(db.clone(), workspace);
        let app = showcase_router(state);
        let (status, _) = post_json(
            &app,
            "POST",
            "/api/showcase/draft",
            json!({
                "theme_id": "classic",
                "composition": empty_showcase_composition(),
                "updated_at": stale,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn showcase_http_lifecycle_save_approve_restore_and_pdf() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let workspace = dir.path().to_path_buf();
        let store = Store::open(&db).unwrap();
        let detail = store
            .create_element(&CreateElementRequest {
                id: "header.j2lifecycle".into(),
                kind: "header".into(),
                payload: json!({
                    "name": "Ada Lovelace",
                    "phone": "703-555-0100",
                    "email": "ada@example.com",
                }),
                parent_element_id: None,
            })
            .unwrap();
        let revision_id = detail.latest.as_ref().unwrap().id.clone();
        let composition =
            header_showcase_composition("header.j2lifecycle", &revision_id);
        let state = mock_state(db.clone(), workspace.clone());
        let app = showcase_router(state);
        let updated_at = store.get_showcase().unwrap().updated_at;
        let (status, saved) = post_json(
            &app,
            "POST",
            "/api/showcase/draft",
            json!({
                "theme_id": "classic",
                "composition": composition,
                "updated_at": updated_at,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let updated_at = saved["updated_at"].as_str().unwrap();
        let (approve_status, first_pub) = post_json(
            &app,
            "POST",
            "/api/showcase/approve",
            json!({ "updated_at": updated_at }),
        )
        .await;
        assert_eq!(approve_status, StatusCode::CREATED, "{first_pub:?}");
        let first_id = first_pub["id"].as_str().unwrap().to_string();
        let pdf_req = Request::builder()
            .method("GET")
            .uri(format!(
                "/api/showcase/publications/{first_id}/resume.pdf"
            ))
            .body(Body::empty())
            .unwrap();
        let pdf_resp = app.clone().oneshot(pdf_req).await.unwrap();
        assert_eq!(pdf_resp.status(), StatusCode::OK);
        let pdf_bytes = axum::body::to_bytes(pdf_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF"));
        let publication = store.get_showcase_publication(&first_id).unwrap().unwrap();
        assert_eq!(publication.pdf_sha256, sha256_prefixed(&pdf_bytes));

        store
            .create_revision(
                "header.j2lifecycle",
                json!({
                    "name": "Ada Lovelace",
                    "phone": "703-555-0199",
                    "email": "ada@example.com",
                }),
            )
            .unwrap();
        let revision2 = store
            .get_element("header.j2lifecycle")
            .unwrap()
            .unwrap()
            .latest
            .as_ref()
            .unwrap()
            .id
            .clone();
        let revised = header_showcase_composition("header.j2lifecycle", &revision2);
        let updated_at = store.get_showcase().unwrap().updated_at;
        let (status, saved2) = post_json(
            &app,
            "POST",
            "/api/showcase/draft",
            json!({
                "theme_id": "classic",
                "composition": revised,
                "updated_at": updated_at,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let draft_before_restore = store.get_showcase().unwrap();
        let (approve2_status, second_pub) = post_json(
            &app,
            "POST",
            "/api/showcase/approve",
            json!({ "updated_at": saved2["updated_at"].as_str().unwrap() }),
        )
        .await;
        assert_eq!(approve2_status, StatusCode::CREATED, "{second_pub:?}");
        let second_id = second_pub["id"].as_str().unwrap();
        assert_ne!(second_id, first_id.as_str());

        let (restore_status, _) = post_json(
            &app,
            "POST",
            "/api/showcase/restore",
            json!({ "publication_id": first_id }),
        )
        .await;
        assert_eq!(restore_status, StatusCode::OK);
        let draft_after = store.get_showcase().unwrap();
        assert_eq!(draft_after.composition, draft_before_restore.composition);
        assert_eq!(draft_after.theme_id, draft_before_restore.theme_id);
        assert_eq!(
            store.get_showcase().unwrap().current_publication_id.as_deref(),
            Some(first_id.as_str())
        );
        let pdf_req = Request::builder()
            .method("GET")
            .uri(format!(
                "/api/showcase/publications/{first_id}/resume.pdf"
            ))
            .body(Body::empty())
            .unwrap();
        let pdf_resp = app.oneshot(pdf_req).await.unwrap();
        assert_eq!(pdf_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn patch_application_theme_only_rejects_unknown_theme() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let workspace = dir.path().to_path_buf();
        let store = Store::open(&db).unwrap();
        let app = store
            .create_application(
                CreateApplicationRequest {
                    company: "Co".into(),
                    title: "Role".into(),
                    posting_url: None,
                    posting_text: None,
                    source: None,
                    status: None,
                    tags: vec![],
                    clone_from_application_id: None,
                    clone_from_preset_id: None,
                    employment_type: None,
                    work_mode: None,
                    location: None,
                    compensation: None,
                    draft_theme_id: None,
                },
                Composition { sections: vec![] },
            )
            .unwrap();
        let state = mock_state(db, workspace);
        let router = applications_router(state);
        let req = Request::builder()
            .method("PATCH")
            .uri(format!("/api/applications/{}", app.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "draft_theme_id": "invalid-theme" })).unwrap(),
            ))
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn preferences_default_theme_persists_in_toml() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let workspace = dir.path().to_path_buf();
        let _store = Store::open(&db).unwrap();
        let state = mock_state(db, workspace.clone());
        let app = showcase_router(state);
        let req = Request::builder()
            .method("POST")
            .uri("/api/preferences/default-theme")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "slot": "application",
                    "theme_id": "ats-classic",
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let prefs_path = workspace.join("preferences.toml");
        let on_disk = std::fs::read_to_string(&prefs_path).unwrap();
        assert!(on_disk.contains("ats-classic"));
    }
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
