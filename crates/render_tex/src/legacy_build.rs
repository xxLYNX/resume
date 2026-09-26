use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use domain::LegacyConfig;
use render_pdf::write_tex_and_compile;
use resolve::ResolvedSections;
use sha2::{Digest, Sha256};
use store::Store;

use crate::build_resume_tex;

pub struct LegacyBuildResult {
    pub tex: String,
    pub tex_path: PathBuf,
    pub pdf_path: Option<PathBuf>,
    pub content_hash: String,
    pub pdf_engine: Option<String>,
}

pub fn jobdesk_build_to_dir(
    store: &Store,
    project_root: &Path,
    config_path: &Path,
    output_dir: &Path,
    compile_pdf: bool,
) -> Result<LegacyBuildResult> {
    let composition = resolve::default_composition(store)?;
    jobdesk_build_composition_to_dir(
        store,
        project_root,
        config_path,
        output_dir,
        &composition,
        compile_pdf,
    )
}

pub fn jobdesk_build_composition_to_dir(
    store: &Store,
    project_root: &Path,
    config_path: &Path,
    output_dir: &Path,
    composition: &domain::Composition,
    compile_pdf: bool,
) -> Result<LegacyBuildResult> {
    let resolved = resolve::resolve_composition(store, composition)?;
    write_build_to_dir(
        project_root,
        config_path,
        output_dir,
        &resolved,
        compile_pdf,
    )
}

pub fn jobdesk_build_resolved_to_dir(
    project_root: &Path,
    config_path: &Path,
    output_dir: &Path,
    resolved: &ResolvedSections,
    compile_pdf: bool,
) -> Result<LegacyBuildResult> {
    write_build_to_dir(
        project_root,
        config_path,
        output_dir,
        resolved,
        compile_pdf,
    )
}

/// Same build targets as `serve` preview: catalog default, application draft, or canonical submission.
pub fn jobdesk_build_aligned(
    store: &Store,
    workspace: &Path,
    project_root: &Path,
    config_path: &Path,
    output_dir: &Path,
    compile_pdf: bool,
    application_id: Option<&str>,
    preview_mode: &str,
) -> Result<LegacyBuildResult> {
    match application_id {
        None => jobdesk_build_to_dir(
            store,
            project_root,
            config_path,
            output_dir,
            compile_pdf,
        ),
        Some(app_id) => match preview_mode {
            "submitted" => {
                let sub = store
                    .get_canonical_submission_with_root(workspace, app_id)?
                    .with_context(|| {
                        format!("no canonical submission for application `{app_id}`")
                    })?;
                let resolved = resolve::resolve_from_manifest(&sub.manifest)?;
                let theme_root = sub
                    .manifest
                    .theme_files_hash
                    .as_deref()
                    .and_then(|h| theme::project_root_for_theme_hash(workspace, h))
                    .unwrap_or_else(|| project_root.to_path_buf());
                jobdesk_build_resolved_to_dir(
                    &theme_root,
                    config_path,
                    output_dir,
                    &resolved,
                    compile_pdf,
                )
            }
            _ => {
                let app = store
                    .get_application(app_id)?
                    .with_context(|| format!("unknown application `{app_id}`"))?;
                jobdesk_build_composition_to_dir(
                    store,
                    project_root,
                    config_path,
                    output_dir,
                    &app.draft_composition,
                    compile_pdf,
                )
            }
        },
    }
}

pub fn write_build_to_dir(
    project_root: &Path,
    config_path: &Path,
    output_dir: &Path,
    resolved: &ResolvedSections,
    compile_pdf: bool,
) -> Result<LegacyBuildResult> {
    let config_text = fs::read_to_string(config_path)
        .with_context(|| format!("read config {}", config_path.display()))?;
    let legacy: LegacyConfig =
        serde_yaml::from_str(&config_text).context("parse build config YAML")?;

    let tex = build_resume_tex(config_path, project_root, resolved)?;
    let content_hash = hash_tex(&tex);

    fs::create_dir_all(output_dir)?;
    let tex_path = output_dir.join(&legacy.output.filename);
    fs::write(&tex_path, &tex)?;

    let mut pdf_path = None;
    let mut pdf_engine = None;

    if compile_pdf {
        match write_tex_and_compile(&tex, output_dir, &legacy.output.filename) {
            Ok(path) => {
                pdf_engine = render_pdf::PdfEngine::detect().map(|e| e.name().to_string());
                pdf_path = Some(path);
            }
            Err(err) => {
                let log_path = output_dir.join("build.log");
                let _ = fs::write(&log_path, format!("{err:#}"));
                return Err(err).context("PDF compilation failed (see build.log in output dir)");
            }
        }
    }

    Ok(LegacyBuildResult {
        tex,
        tex_path,
        pdf_path,
        content_hash,
        pdf_engine,
    })
}

pub fn hash_tex(tex: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tex.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
