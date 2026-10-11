use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use domain::ElementKind;
use serde_json::Value;

use crate::catalog::kind_from_str;
use crate::Store;

const PDF_GROUPS: &[(&str, &str)] = &[
    ("languages", "Tools / languages"),
    ("securityTools", "Tools / security"),
    ("certifications", "Certifications"),
    ("other", "Other"),
];

/// Build markdown inventory from legacy skills blob and catalog technology fields.
pub fn write_legacy_skills_inventory(store: &Store, project_root: &Path, out_path: &Path) -> Result<()> {
    let mut sections: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut sources: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let skills_yaml = project_root.join("content/skills.yaml");
    if skills_yaml.is_file() {
        let raw = std::fs::read_to_string(&skills_yaml)
            .with_context(|| format!("read {}", skills_yaml.display()))?;
        let doc: Value = serde_yaml::from_str(&raw).context("parse skills.yaml")?;
        if let Some(skills) = doc.get("skills").and_then(|v| v.as_object()) {
            for (group, label) in PDF_GROUPS {
                if let Some(line) = skills.get(*group).and_then(|v| v.as_str()) {
                    for token in split_tokens(line) {
                        sections
                            .entry(label.to_string())
                            .or_default()
                            .insert(token.clone());
                        sources
                            .entry(token)
                            .or_default()
                            .push(format!("content/skills.yaml:{group}"));
                    }
                }
            }
        }
    }

    for el in store.list_elements(None)? {
        let Some(kind) = kind_from_str(&el.kind) else {
            continue;
        };
        if !matches!(
            kind,
            ElementKind::Experience | ElementKind::Project | ElementKind::Education
        ) {
            continue;
        }
        let Some(rev_id) = el.latest_revision_id.as_deref() else {
            continue;
        };
        let rev = store
            .get_revision(rev_id)?
            .with_context(|| format!("revision `{rev_id}` missing"))?;
        let obj = rev.payload.as_object();
        let Some(obj) = obj else { continue };
        if let Some(tech) = obj.get("technologies").and_then(|v| v.as_str()) {
            for token in split_tokens(tech) {
                sections
                    .entry("Technologies (from elements)".into())
                    .or_default()
                    .insert(token.clone());
                sources
                    .entry(token)
                    .or_default()
                    .push(format!("element:{}:technologies", el.id));
            }
        }
    }

    let mut md = String::from("# Legacy skills inventory\n\n");
    md.push_str("Review and re-link items on portfolio elements (Tools / Competencies). ");
    md.push_str("Do not treat this file as source of truth.\n\n");

    for (heading, tokens) in &sections {
        md.push_str(&format!("## {heading}\n\n"));
        for token in tokens {
            let src = sources
                .get(token)
                .map(|v| v.join(", "))
                .unwrap_or_default();
            md.push_str(&format!("- {token}"));
            if !src.is_empty() {
                md.push_str(&format!(" — _{src}_"));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    if sections.values().all(|s| s.is_empty()) {
        md.push_str("_No legacy tokens found._\n");
    }

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(out_path, md).with_context(|| format!("write {}", out_path.display()))?;
    Ok(())
}

fn split_tokens(s: &str) -> Vec<String> {
    s.split([',', ';'])
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}
