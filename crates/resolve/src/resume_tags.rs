use anyhow::{Context, Result};
use domain::{Composition, CompositionItem};
use serde_json::Value;
use store::Store;

use crate::config::ResolveConfig;

pub fn validate_composition_resume_tags(
    store: &Store,
    composition: &Composition,
    config: &ResolveConfig,
) -> Result<()> {
    for section in &composition.sections {
        if section.section_key != "projects" {
            continue;
        }
        let max = config
            .resume_line_tags_max
            .get(&section.section_key)
            .copied();
        for item in &section.items {
            validate_item_resume_tags(store, &section.section_key, item, max)?;
        }
    }
    Ok(())
}

fn validate_item_resume_tags(
    store: &Store,
    section_key: &str,
    item: &CompositionItem,
    max: Option<usize>,
) -> Result<()> {
    if item.resume_tag_skill_ids.is_empty() {
        return Ok(());
    }
    if let Some(max) = max {
        if item.resume_tag_skill_ids.len() > max {
            anyhow::bail!(
                "section `{section_key}` element `{}`: at most {max} resume-line tags allowed, got {}",
                item.element_id,
                item.resume_tag_skill_ids.len()
            );
        }
    }
    let asserted = store.asserted_skill_ids_for_element(&item.element_id)?;
    for id in &item.resume_tag_skill_ids {
        if !asserted.contains(id) {
            anyhow::bail!(
                "section `{section_key}` element `{}`: resume tag `{id}` is not an asserted evidence link on this entry",
                item.element_id
            );
        }
    }
    Ok(())
}

pub fn inject_project_resume_tags(
    store: &Store,
    items: &[CompositionItem],
    payloads: &mut [Value],
) -> Result<()> {
    for (item, payload) in items.iter().zip(payloads.iter_mut()) {
        let Some(obj) = payload.as_object_mut() else {
            continue;
        };
        obj.remove("technologies");
        let resume_tags = labels_for_resume_tag_ids(store, item)?;
        if resume_tags.is_empty() {
            obj.remove("resume_tags");
        } else {
            obj.insert("resume_tags".into(), Value::String(resume_tags));
        }
    }
    Ok(())
}

fn labels_for_resume_tag_ids(store: &Store, item: &CompositionItem) -> Result<String> {
    if item.resume_tag_skill_ids.is_empty() {
        return Ok(String::new());
    }
    let mut parts = Vec::with_capacity(item.resume_tag_skill_ids.len());
    for id in &item.resume_tag_skill_ids {
        let node = store
            .get_skill_node(id)
            .with_context(|| format!("missing skill `{id}`"))?
            .ok_or_else(|| anyhow::anyhow!("missing skill `{id}`"))?;
        parts.push(node.label);
    }
    Ok(parts.join(", "))
}
