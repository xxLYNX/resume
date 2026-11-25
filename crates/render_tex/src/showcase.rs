use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use domain::{showcase_composition_is_empty, ShowcasePublication};
use resolve::{resolve_composition_with_config, ResolveConfig};
use store::Store;
use theme::{
    ensure_theme_snapshot, hash_theme_dir, load_theme_info, project_root_for_theme_hash,
    theme_dir_for_id,
};

use crate::legacy_build::write_build_with_config;
use crate::{load_themed_build_config, section_render_order};

static BUILD_SEQ: AtomicU64 = AtomicU64::new(0);

/// Approve the captured Showcase draft.
///
/// One read of the draft supplies the composition, theme, and resolved data.
/// The publication commit rejects the attempt if that draft row changed.
pub fn approve_showcase(
    store: &Store,
    project_root: &Path,
    workspace: &Path,
) -> Result<ShowcasePublication> {
    let captured = store.capture_showcase()?;
    if showcase_composition_is_empty(&captured.composition) {
        bail!("showcase draft has no pinned elements — publication was not created");
    }
    let theme_files_hash = ensure_theme_snapshot(
        project_root,
        &captured.theme_id,
        &workspace.join("artifacts/theme_snapshots"),
    )
    .with_context(|| format!("snapshot showcase theme `{}`", captured.theme_id))?;
    let snapshot_root =
        project_root_for_theme_hash(workspace, &theme_files_hash).with_context(|| {
            format!(
                "theme snapshot for `{}` is missing after capture",
                captured.theme_id
            )
        })?;
    verify_theme_snapshot_hash(&snapshot_root, &captured.theme_id, &theme_files_hash)?;
    let theme = load_theme_info(&snapshot_root, &captured.theme_id)
        .with_context(|| format!("load showcase theme `{}`", captured.theme_id))?;
    let mut render_composition = captured.composition.clone();
    render_composition
        .sections
        .retain(|section| !section.items.is_empty());
    let resolved_map = resolve_composition_with_config(
        store,
        &render_composition,
        &ResolveConfig::from_theme(&theme),
    )
    .context("showcase approval did not create a publication")?;
    let resolved: BTreeMap<_, _> = resolved_map
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let config_path = project_root.join("settings/jobdesk.yaml");
    let (mut config, month_display) =
        load_themed_build_config(&snapshot_root, &config_path, &captured.theme_id)?;
    config.output.filename = "resume.tex".to_string();
    let order: Vec<String> = captured
        .composition
        .sections
        .iter()
        .map(|section| section.section_key.clone())
        .collect();
    let enabled = section_render_order(&config, Some(&order));
    let pdf_bytes = {
        let build = UniqueBuild::create(workspace)?;
        let built = write_build_with_config(
            &snapshot_root,
            &config,
            month_display,
            build.path(),
            &resolved_map,
            true,
            Some(&order),
        )
        .context("showcase approval produced no publication")?;
        let Some(pdf_path) = built.pdf_path else {
            bail!("showcase approval produced no PDF — publication was not created");
        };
        let bytes = fs::read(&pdf_path).with_context(|| format!("read {}", pdf_path.display()))?;
        if bytes.is_empty() {
            bail!("showcase approval compiled no PDF pages — publication was not created");
        }
        bytes
    };
    store.approve_showcase_snapshot(
        workspace,
        &captured,
        &resolved,
        &pdf_bytes,
        &theme_files_hash,
        &enabled,
    )
}

fn verify_theme_snapshot_hash(snapshot_root: &Path, theme_id: &str, expected: &str) -> Result<()> {
    let actual = hash_theme_dir(&theme_dir_for_id(snapshot_root, theme_id))?;
    if actual != expected {
        bail!(
            "recorded theme hash {expected} does not describe the theme files used for rendering ({actual})"
        );
    }
    Ok(())
}

struct UniqueBuild {
    path: PathBuf,
}

impl UniqueBuild {
    fn create(workspace: &Path) -> Result<Self> {
        let seq = BUILD_SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = workspace.join(format!(
            "artifacts/showcase-build/{}.{}.{}",
            std::process::id(),
            nanos,
            seq
        ));
        fs::create_dir_all(&path)
            .with_context(|| format!("create showcase build dir {}", path.display()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for UniqueBuild {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_build::{write_build_to_dir_named, write_build_with_config};
    use domain::{
        compute_showcase_snapshot_hash, empty_showcase_composition, Composition, CompositionItem,
        CompositionSection,
    };
    use render_pdf::PdfEngine;
    use serde_json::json;
    use std::path::PathBuf;
    use store::CreateElementRequest;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn header_composition(element_id: &str, revision_id: &str) -> Composition {
        Composition {
            sections: vec![CompositionSection {
                section_key: "header".into(),
                items: vec![CompositionItem {
                    element_id: element_id.into(),
                    revision_id: revision_id.into(),
                    resume_tag_skill_ids: Vec::new(),
                }],
            }],
        }
    }

    fn enabled_section_names(yaml: &str) -> Vec<String> {
        let config: domain::LegacyConfig = serde_yaml::from_str(yaml).unwrap();
        let order: Vec<String> = config
            .sections
            .iter()
            .map(|section| section.name.clone())
            .collect();
        section_render_order(&config, Some(&order))
    }

    #[test]
    fn yaml_filename_and_comments_do_not_change_enabled_sections() {
        let base = fs::read_to_string(repo_root().join("settings/jobdesk.yaml")).unwrap();
        let commented = format!("# extra comment\n{base}");
        let renamed = base.replace("filename: \"resume.tex\"", "filename: \"other.tex\"");
        let disabled = base.replacen("enabled: true", "enabled: false", 1);
        let base_names = enabled_section_names(&base);
        assert_eq!(base_names, enabled_section_names(&commented));
        assert_eq!(base_names, enabled_section_names(&renamed));
        assert_ne!(base_names, enabled_section_names(&disabled));
        assert!(base_names.iter().any(|name| name == "header"));
        assert!(enabled_section_names(&disabled)
            .iter()
            .all(|name| name != "header"));
    }

    #[test]
    fn captured_config_drives_identity_and_rendering() {
        let root = repo_root();
        let (mut config, month_display) =
            load_themed_build_config(&root, &root.join("settings/jobdesk.yaml"), "classic")
                .unwrap();
        config.output.filename = "resume.tex".to_string();
        let order: Vec<String> = config
            .sections
            .iter()
            .map(|section| section.name.clone())
            .collect();
        let enabled = section_render_order(&config, Some(&order));
        let mut without_header = config.clone();
        for section in &mut without_header.sections {
            if section.name == "header" {
                section.enabled = false;
            }
        }
        let disabled = section_render_order(&without_header, Some(&order));
        assert_ne!(enabled, disabled);
        let composition = empty_showcase_composition();
        let resolved = BTreeMap::new();
        let with_header = compute_showcase_snapshot_hash(
            "classic",
            "sha256:theme",
            &enabled,
            &composition,
            &resolved,
        );
        let header_off = compute_showcase_snapshot_hash(
            "classic",
            "sha256:theme",
            &disabled,
            &composition,
            &resolved,
        );
        assert_ne!(with_header, header_off);

        let mut sections = resolve::ResolvedSections::new();
        sections.insert("header".into(), json!({"name": "Captured Header Marker"}));
        let workspace = tempfile::tempdir().unwrap();
        let enabled_dir = workspace.path().join("enabled");
        let disabled_dir = workspace.path().join("disabled");
        let enabled_tex = write_build_with_config(
            &root,
            &config,
            month_display,
            &enabled_dir,
            &sections,
            false,
            Some(&order),
        )
        .unwrap();
        let disabled_tex = write_build_with_config(
            &root,
            &without_header,
            month_display,
            &disabled_dir,
            &sections,
            false,
            Some(&order),
        )
        .unwrap();
        assert!(enabled_tex.tex.contains("Captured Header Marker"));
        assert!(!disabled_tex.tex.contains("Captured Header Marker"));
        assert_eq!(
            enabled,
            section_render_order(&config, Some(&order)),
            "rendering must keep the configuration used for the snapshot hash"
        );
    }

    #[test]
    fn tampered_theme_snapshot_is_rejected() {
        let workspace = tempfile::tempdir().unwrap();
        let hash = ensure_theme_snapshot(
            &repo_root(),
            "classic",
            &workspace.path().join("artifacts/theme_snapshots"),
        )
        .unwrap();
        let root = project_root_for_theme_hash(workspace.path(), &hash).unwrap();
        verify_theme_snapshot_hash(&root, "classic", &hash).unwrap();
        let preamble = theme_dir_for_id(&root, "classic").join("preamble.tex");
        fs::write(&preamble, b"% tampered\n").unwrap();
        let err = verify_theme_snapshot_hash(&root, "classic", &hash).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("does not describe"), "{msg}");
    }

    #[test]
    fn two_build_directories_do_not_collide_and_are_removed() {
        let workspace = tempfile::tempdir().unwrap();
        let first = UniqueBuild::create(workspace.path()).unwrap();
        let second = UniqueBuild::create(workspace.path()).unwrap();
        assert_ne!(first.path(), second.path());
        fs::write(first.path().join("resume.pdf"), b"one").unwrap();
        fs::write(second.path().join("resume.pdf"), b"two").unwrap();
        let first_path = first.path().to_path_buf();
        let second_path = second.path().to_path_buf();
        drop(first);
        assert!(!first_path.exists());
        assert_eq!(fs::read(second_path.join("resume.pdf")).unwrap(), b"two");
        let failed = UniqueBuild::create(workspace.path()).unwrap();
        let failed_path = failed.path().to_path_buf();
        let built = write_build_to_dir_named(
            &repo_root(),
            &workspace.path().join("missing.yaml"),
            failed.path(),
            &resolve::ResolvedSections::new(),
            false,
            "classic",
            None,
            Some("resume.tex"),
        );
        let message = match built {
            Ok(_) => panic!("missing build config must fail"),
            Err(err) => format!("{err:#}"),
        };
        drop(failed);
        assert!(!failed_path.exists(), "{message}");
        drop(second);
        assert!(!second_path.exists());
        let build_root = workspace.path().join("artifacts/showcase-build");
        if build_root.exists() {
            assert!(fs::read_dir(&build_root).unwrap().next().is_none());
        }
    }

    #[test]
    fn missing_theme_does_not_create_a_publication() {
        let workspace = tempfile::tempdir().unwrap();
        let store = Store::open(&workspace.path().join("jobdesk.db")).unwrap();
        store
            .save_showcase_draft(
                "missing-theme",
                &header_composition("header.missing", "rev-missing"),
            )
            .unwrap();
        let err = approve_showcase(&store, &repo_root(), workspace.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("missing-theme"), "{msg}");
        assert!(store.list_showcase_publications().unwrap().is_empty());
        assert!(store
            .get_showcase()
            .unwrap()
            .current_publication_id
            .is_none());
        assert!(!workspace.path().join("artifacts/showcase").exists());
    }

    #[test]
    fn dangling_draft_pin_does_not_create_a_publication() {
        let workspace = tempfile::tempdir().unwrap();
        let store = Store::open(&workspace.path().join("jobdesk.db")).unwrap();
        store
            .save_showcase_draft(
                "classic",
                &header_composition("header.missing", "rev-missing"),
            )
            .unwrap();
        let err = approve_showcase(&store, &repo_root(), workspace.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("rev-missing") || msg.contains("resolve") || msg.contains("revision"),
            "{msg}"
        );
        assert!(store.list_showcase_publications().unwrap().is_empty());
        assert!(store
            .get_showcase()
            .unwrap()
            .current_publication_id
            .is_none());
        assert_no_build_pdf(workspace.path());
    }

    #[test]
    fn empty_draft_is_rejected_before_rendering() {
        let workspace = tempfile::tempdir().unwrap();
        let store = Store::open(&workspace.path().join("jobdesk.db")).unwrap();
        let err = approve_showcase(&store, &repo_root(), workspace.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("no pinned"), "{msg}");
        assert!(store.list_showcase_publications().unwrap().is_empty());
        assert!(store
            .get_showcase()
            .unwrap()
            .current_publication_id
            .is_none());
        assert!(!workspace.path().join("artifacts/showcase").exists());
        assert!(!workspace.path().join("artifacts/showcase-build").exists());
    }

    #[test]
    fn pinned_header_approval_freezes_pdf_bytes() {
        let workspace = tempfile::tempdir().unwrap();
        let store = Store::open(&workspace.path().join("jobdesk.db")).unwrap();
        let element = store
            .create_element(&CreateElementRequest {
                id: "header.showcase".into(),
                kind: "header".into(),
                payload: json!({"name": "Ada Lovelace"}),
                parent_element_id: None,
            })
            .unwrap();
        store
            .save_showcase_draft(
                "classic",
                &header_composition(&element.id, &element.revisions[0].id),
            )
            .unwrap();
        let result = approve_showcase(&store, &repo_root(), workspace.path());
        if PdfEngine::detect().is_none() {
            let err = result.expect_err("approval without a PDF engine must fail");
            let msg = format!("{err:#}");
            assert!(msg.contains("publication") || msg.contains("PDF"), "{msg}");
            assert!(store.list_showcase_publications().unwrap().is_empty());
            assert_no_build_pdf(workspace.path());
            return;
        }
        let publication =
            result.expect("header draft should approve when a PDF engine is installed");
        let path = workspace.path().join(&publication.pdf_relpath);
        let bytes = fs::read(&path).expect("frozen PDF");
        assert!(bytes.starts_with(b"%PDF"), "frozen file is not a PDF");
        assert_eq!(publication.pdf_sha256, domain::sha256_prefixed(&bytes));
        assert_eq!(
            publication.resolved["header"]["name"],
            json!("Ada Lovelace")
        );
        let theme_hash = publication
            .theme_files_hash
            .as_deref()
            .expect("publication records a theme hash");
        let snapshot_root =
            project_root_for_theme_hash(workspace.path(), theme_hash).expect("theme snapshot");
        let actual = hash_theme_dir(&theme_dir_for_id(&snapshot_root, "classic")).unwrap();
        assert_eq!(
            publication.theme_files_hash.as_deref(),
            Some(actual.as_str())
        );
        store
            .create_revision("header.showcase", json!({"name": "Changed Later"}))
            .unwrap();
        let stored = store
            .get_showcase_publication(&publication.id)
            .unwrap()
            .unwrap();
        assert_eq!(stored.resolved["header"]["name"], json!("Ada Lovelace"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_no_build_pdf(workspace.path());
    }

    fn assert_no_build_pdf(workspace: &Path) {
        let root = workspace.join("artifacts/showcase-build");
        if !root.exists() {
            return;
        }
        for entry in fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            assert!(!path.join("resume.pdf").exists(), "{}", path.display());
        }
    }
}
