use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

pub const CLASSIC_THEME_DIR: &str = "themes/classic";

#[derive(Debug, Clone, Serialize)]
pub struct ThemeInfo {
    pub id: String,
    pub version: u64,
    pub description: String,
    pub contract_version: u64,
    /// `abbrev` (e.g. Dec.) or `numeric` (e.g. 12) — jobdesk-owned month tokens, theme picks style.
    pub month_display: String,
    pub files_hash: String,
    pub sections: BTreeMap<String, ThemeSectionInfo>,
    /// MiniJinja template name → path relative to theme root (`macros/...`).
    pub macros: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeSectionInfo {
    pub template: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_fields: Vec<String>,
    /// Which calendar parts row dates use for display (`month`, `year`, optional `day`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub date_parts: Vec<String>,
    /// Catalog element kind for this section (`project` for `projects`).
    pub element_kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogFieldDef {
    pub key: String,
    pub label: String,
    pub field_type: String,
    pub optional: bool,
    /// For `date_range` / `date` fields: theme-driven editor + render precision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub date_parts: Vec<String>,
    /// For `date_range`: `abbrev` or `numeric` (matches theme `month_display`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub month_display: String,
}

/// Stable hash of all files under the theme directory (sorted paths, per-file sha256).
pub fn hash_theme_dir(theme_dir: &Path) -> Result<String> {
    let mut files: Vec<(String, String)> = Vec::new();
    for entry in WalkDir::new(theme_dir).sort_by_file_name() {
        let entry = entry.context("walk theme dir")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(theme_dir)
            .context("strip theme prefix")?
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(entry.path())
            .with_context(|| format!("read {}", entry.path().display()))?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        files.push((rel, format!("{:x}", hasher.finalize())));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let body = serde_json::json!({ "files": files });
    let json = serde_json::to_string(&body).expect("theme hash body serializes");
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub fn theme_dir_for_id(project_root: &Path, theme_id: &str) -> PathBuf {
    project_root.join(format!("themes/{theme_id}"))
}

pub fn load_theme_info(project_root: &Path, theme_id: &str) -> Result<ThemeInfo> {
    let theme_dir = theme_dir_for_id(project_root, theme_id);
    let toml_path = theme_dir.join("theme.toml");
    let raw = fs::read_to_string(&toml_path)
        .with_context(|| format!("read {}", toml_path.display()))?;
    let doc: toml::Table = toml::from_str(&raw).context("parse theme.toml")?;

    let id = doc
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(theme_id)
        .to_string();
    let version = doc
        .get("version")
        .and_then(|v| v.as_integer())
        .unwrap_or(1) as u64;
    let description = doc
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let contract_version = doc
        .get("contract_version")
        .and_then(|v| v.as_integer())
        .unwrap_or(1) as u64;
    let month_display = doc
        .get("month_display")
        .and_then(|v| v.as_str())
        .unwrap_or("abbrev")
        .to_string();

    let mut macros = BTreeMap::new();
    if let Some(table) = doc.get("macros").and_then(|v| v.as_table()) {
        for (name, value) in table {
            if let Some(path) = value.as_str() {
                macros.insert(name.clone(), path.to_string());
            }
        }
    }

    let mut sections = BTreeMap::new();
    if let Some(sec_table) = doc.get("sections").and_then(|v| v.as_table()) {
        for (key, value) in sec_table {
            let Some(sec) = value.as_table() else {
                continue;
            };
            let template = sec
                .get("template")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let optional_fields: Vec<String> = sec
                .get("optional_fields")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let date_parts: Vec<String> = sec
                .get("date_parts")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_else(|| vec!["month".into(), "year".into()]);
            sections.insert(
                key.clone(),
                ThemeSectionInfo {
                    template,
                    optional_fields,
                    date_parts,
                    element_kind: section_element_kind(key),
                },
            );
        }
    }

    let files_hash = hash_theme_dir(&theme_dir)?;

    Ok(ThemeInfo {
        id,
        version,
        description,
        contract_version,
        month_display,
        files_hash,
        sections,
        macros,
    })
}

fn section_element_kind(section_key: &str) -> String {
    match section_key {
        "projects" => "project".to_string(),
        other => other.to_string(),
    }
}

/// Copy `themes/{theme_id}/` into `dest_root/themes/{theme_id}/` (idempotent).
pub fn ensure_theme_snapshot(project_root: &Path, theme_id: &str, dest_root: &Path) -> Result<String> {
    let theme_dir = theme_dir_for_id(project_root, theme_id);
    let files_hash = hash_theme_dir(&theme_dir)?;
    let snapshot_root = dest_root.join(files_hash.strip_prefix("sha256:").unwrap_or(&files_hash));
    let target = snapshot_root.join(format!("themes/{theme_id}"));
    if target.join("theme.toml").exists() {
        return Ok(files_hash);
    }
    fs::create_dir_all(&target)?;
    for entry in WalkDir::new(&theme_dir).sort_by_file_name() {
        let entry = entry.context("walk theme")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(&theme_dir).context("strip")?;
        let out_path = target.join(rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(entry.path(), &out_path)
            .with_context(|| format!("copy {}", entry.path().display()))?;
    }
    Ok(files_hash)
}

/// Project root that contains `themes/{theme_id}/` for a pinned snapshot hash.
pub fn project_root_for_theme_hash(workspace: &Path, files_hash: &str) -> Option<PathBuf> {
    let key = files_hash.strip_prefix("sha256:").unwrap_or(files_hash);
    let root = workspace.join("artifacts/theme_snapshots").join(key);
    if root.join("themes").exists() {
        Some(root)
    } else {
        None
    }
}

pub fn catalog_fields_for_kind(kind: &str, theme: &ThemeInfo) -> Vec<CatalogFieldDef> {
    let section_key = match kind {
        "project" => "projects",
        other => other,
    };

    let section = theme.sections.get(section_key);
    let optional: std::collections::HashSet<String> = section
        .map(|s| s.optional_fields.iter().cloned().collect())
        .unwrap_or_default();
    let date_parts: Vec<String> = section
        .map(|s| s.date_parts.clone())
        .unwrap_or_else(|| vec!["month".into(), "year".into()]);
    let month_display = theme.month_display.clone();

    let base = base_fields_for_kind(kind);
    base.into_iter()
        .map(|mut f| {
            if optional.contains(&f.key) {
                f.optional = true;
            }
            if f.field_type == "date_range" || f.field_type == "date" {
                f.date_parts = date_parts.clone();
                f.month_display = month_display.clone();
            }
            f
        })
        .collect()
}

fn base_fields_for_kind(kind: &str) -> Vec<CatalogFieldDef> {
    match kind {
        "header" => vec![
            field("name", "Name", "string", false),
            field("location", "Location", "string", true),
            field("email", "Email", "string", true),
            field("phone", "Phone", "string", true),
            field("linkedin", "LinkedIn", "string", true),
            field("github", "GitHub", "string", true),
        ],
        "objective" => vec![field("objective", "Objective", "text", false)],
        "education" => vec![
            field("school", "School", "string", false),
            field("degree", "Degree", "string", false),
            field("date_range", "Dates", "date_range", true),
            field("highlights", "Highlights", "string_list", true),
            field("gpa", "GPA", "string", true),
        ],
        "experience" => vec![
            field("company", "Company", "string", false),
            field("location", "Location", "work_location", true),
            field("position", "Position", "string", false),
            field("date_range", "Dates", "date_range", true),
            field("highlights", "Highlights", "string_list", true),
        ],
        "project" => vec![
            field("name", "Name", "string", false),
            field("technologies", "Technologies", "string", true),
            field("date_range", "Dates", "date_range", true),
            field("details", "Details", "string_list", true),
        ],
        "skills" => vec![
            field("skills.languages", "Languages", "string", true),
            field("skills.securityTools", "Security tools", "string", true),
            field("skills.certifications", "Certifications", "string", true),
            field("skills.other", "Other", "string", true),
        ],
        _ => Vec::new(),
    }
}

fn field(key: &str, label: &str, field_type: &str, optional: bool) -> CatalogFieldDef {
    CatalogFieldDef {
        key: key.to_string(),
        label: label.to_string(),
        field_type: field_type.to_string(),
        optional,
        date_parts: Vec::new(),
        month_display: String::new(),
    }
}

/// Installed theme bundle ids under `themes/`.
pub fn list_theme_ids(project_root: &Path) -> Result<Vec<String>> {
    let themes_dir = project_root.join("themes");
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(&themes_dir).with_context(|| format!("read {}", themes_dir.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if entry.path().join("theme.toml").is_file() {
            ids.push(id);
        }
    }
    ids.sort();
    Ok(ids)
}
