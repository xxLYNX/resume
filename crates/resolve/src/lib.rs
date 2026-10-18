mod config;
mod normalize;
mod resume_tags;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use config::ResolveConfig;
pub use resume_tags::validate_composition_resume_tags;

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
        ("certifications", ElementKind::Certification),
        ("awards", ElementKind::Award),
    ];
    // Order matches settings/jobdesk.yaml / classic theme (toolbelt is resolve-injected, not composable).

    let mut sections = Vec::new();
    for (section_key, kind) in mapping {
        let mut elements = store.list_elements(Some(kind.as_str()))?;
        elements.sort_by(|a, b| b.timeline_sort_key.cmp(&a.timeline_sort_key));
        let mut items = Vec::new();
        for el in elements {
            let Some(rev_id) = el.latest_revision_id else {
                continue;
            };
            items.push(CompositionItem {
                element_id: el.id,
                revision_id: rev_id,
                resume_tag_skill_ids: Vec::new(),
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
    resolve_composition_with_config(store, composition, &ResolveConfig::default())
}

/// Resolve with theme-driven limits on composition resume-line tag picks.
pub fn resolve_composition_with_config(
    store: &Store,
    composition: &Composition,
    config: &ResolveConfig,
) -> Result<ResolvedSections> {
    resume_tags::validate_composition_resume_tags(store, composition, config)?;

    let mut out = ResolvedSections::new();

    for section in &composition.sections {
        if section.section_key == "skills" || section.section_key == "toolbelt" {
            continue;
        }
        let ctx = if section.section_key == "education" {
            let payloads = load_education_payloads_with_courses(store, &section.items)?;
            normalize::normalize_section_value(
                &section.section_key,
                section_context(&section.section_key, payloads)?,
            )
        } else if section.section_key == "projects" {
            let mut payloads = load_item_payloads(store, &section.items)?;
            resume_tags::inject_project_resume_tags(store, &section.items, &mut payloads)?;
            normalize::normalize_section_value(
                &section.section_key,
                section_context(&section.section_key, payloads)?,
            )
        } else {
            let payloads = load_item_payloads(store, &section.items)?;
            normalize::normalize_section_value(
                &section.section_key,
                section_context(&section.section_key, payloads)?,
            )
        };
        out.insert(section.section_key.clone(), ctx);
    }

    inject_toolbelt_from_evidence(store, composition, &mut out)?;

    Ok(out)
}

fn inject_toolbelt_from_evidence(
    store: &Store,
    composition: &Composition,
    resolved: &mut ResolvedSections,
) -> Result<()> {
    let tags = store.aggregate_toolbelt_tags(composition)?;
    if tags.is_empty() {
        return Ok(());
    }
    resolved.insert("toolbelt".into(), json!({ "tags": tags }));
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

fn load_education_payloads_with_courses(
    store: &Store,
    items: &[CompositionItem],
) -> Result<Vec<Value>> {
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
        let mut payload = rev.payload;
        let courses = course_payloads_under_education(store, &item.element_id)?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("courses".to_string(), json!(courses));
        }
        payloads.push(payload);
    }
    Ok(payloads)
}

fn course_payloads_under_education(store: &Store, education_id: &str) -> Result<Vec<Value>> {
    let courses = store.list_elements(Some("course"))?;
    let mut out = Vec::new();
    for el in courses {
        if el.parent_element_id.as_deref() != Some(education_id) {
            continue;
        }
        let Some(rev_id) = el.latest_revision_id else {
            continue;
        };
        let rev = store
            .get_revision(&rev_id)?
            .with_context(|| format!("course revision for `{}`", el.id))?;
        out.push(rev.payload);
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
        "certifications" => Ok(json!({ "certifications": payloads })),
        "awards" => Ok(json!({ "awards": payloads })),
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
