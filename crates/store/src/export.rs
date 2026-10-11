use std::path::Path;

use anyhow::{Context, Result};
use domain::ElementKind;
use serde_json::json;

use super::catalog::RevisionDetail;
use super::Store;

/// Default relative path under workspace for exported YAML (never overwrites repo `content/`).
pub const STAGING_CONTENT_DIR: &str = "staging/content";

/// Write latest element revisions to a directory (typically `workspace/staging/content/`).
pub fn export_catalog_to_content(store: &Store, content_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(content_dir)?;

    write_singleton(store, content_dir, "header.main", ElementKind::Header, "header.yaml")?;
    write_singleton(
        store,
        content_dir,
        "objective.main",
        ElementKind::Objective,
        "objective.yaml",
    )?;
    write_list(
        store,
        content_dir,
        ElementKind::Education,
        "education",
        "education.yaml",
    )?;
    write_list(
        store,
        content_dir,
        ElementKind::Experience,
        "experience",
        "experience.yaml",
    )?;
    write_list(
        store,
        content_dir,
        ElementKind::Project,
        "projects",
        "projects.yaml",
    )?;

    Ok(())
}

fn write_singleton(
    store: &Store,
    content_dir: &Path,
    element_id: &str,
    _kind: ElementKind,
    filename: &str,
) -> Result<()> {
    let Some(rev) = latest_revision(store, element_id)? else {
        return Ok(());
    };
    let yaml = serde_yaml::to_string(&rev.payload).context("serialize singleton yaml")?;
    std::fs::write(content_dir.join(filename), yaml)?;
    Ok(())
}

fn write_list(
    store: &Store,
    content_dir: &Path,
    kind: ElementKind,
    list_key: &str,
    filename: &str,
) -> Result<()> {
    let elements = store.list_elements(Some(kind.as_str()))?;
    let mut items = Vec::new();
    for el in elements {
        let Some(rev) = latest_revision(store, &el.id)? else {
            continue;
        };
        items.push(rev.payload);
    }
    let root = json!({ list_key: items });
    let yaml = serde_yaml::to_string(&root).context("serialize list yaml")?;
    std::fs::write(content_dir.join(filename), yaml)?;
    Ok(())
}

fn latest_revision(store: &Store, element_id: &str) -> Result<Option<RevisionDetail>> {
    let Some(detail) = store.get_element(element_id)? else {
        return Ok(None);
    };
    Ok(detail.latest)
}
