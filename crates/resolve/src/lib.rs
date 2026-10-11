mod normalize;

use std::collections::HashMap;

use anyhow::{Context, Result};
use domain::{Composition, CompositionItem, CompositionSection, ElementKind, SubmissionManifest};
use serde_json::{json, Value};
use store::Store;

/// Template context per config section name (`header`, `education`, …).
pub type ResolvedSections = HashMap<String, Value>;

/// Build the default “full resume” composition from latest revisions in the catalog.
pub fn default_composition(store: &Store) -> Result<Composition> {
    let mapping = [
        ("header", ElementKind::Header),
        ("objective", ElementKind::Objective),
        ("education", ElementKind::Education),
        ("experience", ElementKind::Experience),
        ("projects", ElementKind::Project),
    ];

    let mut sections = Vec::new();
    for (section_key, kind) in mapping {
        let elements = store.list_elements(Some(kind.as_str()))?;
        let mut items = Vec::new();
        for el in elements {
            let Some(rev_id) = el.latest_revision_id else {
                continue;
            };
            items.push(CompositionItem {
                element_id: el.id,
                revision_id: rev_id,
            });
        }
        if !items.is_empty() {
            sections.push(CompositionSection {
                section_key: section_key.to_string(),
                items,
            });
        }
    }

    Ok(Composition { sections })
}

/// Resolve pinned revisions into MiniJinja contexts keyed by section name.
pub fn resolve_composition(store: &Store, composition: &Composition) -> Result<ResolvedSections> {
    let mut out = ResolvedSections::new();

    for section in &composition.sections {
        if section.section_key == "skills" {
            continue;
        }
        let payloads = load_item_payloads(store, &section.items)?;
        let ctx = normalize::normalize_section_value(
            &section.section_key,
            section_context(&section.section_key, payloads)?,
        );
        out.insert(section.section_key.clone(), ctx);
    }

    inject_skills_from_evidence(store, composition, &mut out)?;

    Ok(out)
}

fn inject_skills_from_evidence(
    store: &Store,
    composition: &Composition,
    resolved: &mut ResolvedSections,
) -> Result<()> {
    let aggregated = store.aggregate_skills_pdf_context(composition)?;
    let skills_obj = aggregated.get("skills").cloned().unwrap_or(json!({}));
    resolved.insert("skills".into(), json!({ "skills": skills_obj }));
    Ok(())
}

/// Re-render from an immutable submission manifest (interview prep path).
pub fn resolve_from_manifest(manifest: &SubmissionManifest) -> Result<ResolvedSections> {
    if manifest.schema_version != domain::MANIFEST_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported manifest schema_version {} (expected {})",
            manifest.schema_version,
            domain::MANIFEST_SCHEMA_VERSION
        );
    }
    let mut out = ResolvedSections::new();
    for (key, value) in &manifest.resolved {
        out.insert(key.clone(), value.clone());
    }
    Ok(out)
}

fn load_item_payloads(store: &Store, items: &[CompositionItem]) -> Result<Vec<Value>> {
    let mut payloads = Vec::with_capacity(items.len());
    for item in items {
        let rev = store
            .get_revision(&item.revision_id)?
            .with_context(|| format!("revision `{}` not found", item.revision_id))?;
        if rev.element_id != item.element_id {
            anyhow::bail!(
                "revision `{}` belongs to `{}`, expected `{}`",
                item.revision_id,
                rev.element_id,
                item.element_id
            );
        }
        payloads.push(rev.payload);
    }
    Ok(payloads)
}

fn section_context(section_key: &str, payloads: Vec<Value>) -> Result<Value> {
    match section_key {
        "header" => {
            let payload = one_payload(section_key, payloads)?;
            Ok(payload)
        }
        "objective" | "skills" => {
            let payload = one_payload(section_key, payloads)?;
            Ok(payload)
        }
        "education" => Ok(json!({ "education": payloads })),
        "experience" => Ok(json!({ "experience": payloads })),
        "projects" => Ok(json!({ "projects": payloads })),
        other => anyhow::bail!("unknown section key `{other}`"),
    }
}

fn one_payload(section_key: &str, payloads: Vec<Value>) -> Result<Value> {
    match payloads.len() {
        0 => anyhow::bail!("section `{section_key}` has no items"),
        1 => Ok(payloads.into_iter().next().unwrap()),
        _ => anyhow::bail!("section `{section_key}` expects one element, got {}", payloads.len()),
    }
}
