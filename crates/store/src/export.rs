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

    write_singleton(
        store,
        content_dir,
        "header.main",
        ElementKind::Header,
        "header.yaml",
    )?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CreateElementRequest;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn export_projects_includes_portfolio_narrative() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        store
            .create_element(&CreateElementRequest {
                id: "project.export-narrative".into(),
                kind: "project".into(),
                payload: serde_json::json!({
                    "name": "Export me",
                    "portfolio_narrative": "Exported narrative body"
                }),
                parent_element_id: None,
            })
            .unwrap();

        let export_dir = dir.path().join("staging/content");
        export_catalog_to_content(&store, &export_dir).unwrap();
        let yaml = std::fs::read_to_string(export_dir.join("projects.yaml")).unwrap();
        assert!(yaml.contains("portfolio_narrative"));
        assert!(yaml.contains("Exported narrative body"));
    }

    #[test]
    fn yaml_export_import_roundtrip_preserves_portfolio_narrative_exactly() {
        use crate::{import_legacy_content, ImportOptions};

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let narrative = concat!(
            "## History\n\n",
            "Line with \"quotes\", [link](https://example.com/path?q=1), and `<tags>`.\n",
            "Backslash: \\\\ end.\n",
            "Unicode: café — em dash.\n",
        );

        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        store
            .create_element(&CreateElementRequest {
                id: "project.yaml-roundtrip".into(),
                kind: "project".into(),
                payload: serde_json::json!({
                    "name": "Yaml Roundtrip",
                    "details": ["bullet"],
                    "portfolio_narrative": narrative
                }),
                parent_element_id: None,
            })
            .unwrap();

        let export_dir = dir.path().join("staging/content");
        export_catalog_to_content(&store, &export_dir).unwrap();

        let content_dir = dir.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();
        for name in [
            "header.yaml",
            "objective.yaml",
            "education.yaml",
            "experience.yaml",
        ] {
            std::fs::copy(repo_root.join("content").join(name), content_dir.join(name)).unwrap();
        }
        std::fs::copy(
            export_dir.join("projects.yaml"),
            content_dir.join("projects.yaml"),
        )
        .unwrap();

        let db2_path = dir.path().join("jobdesk-import.db");
        let store2 = Store::open(&db2_path).unwrap();
        import_legacy_content(dir.path(), &store2, ImportOptions::default()).unwrap();

        let imported = store2
            .get_element("project.yaml-roundtrip")
            .unwrap()
            .unwrap()
            .latest
            .unwrap();
        assert_eq!(
            imported
                .payload
                .get("portfolio_narrative")
                .unwrap()
                .as_str()
                .unwrap(),
            narrative
        );
    }
}
