use std::collections::BTreeSet;

use serde::Serialize;

use crate::SkillNode;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillPostingMatch {
    pub skill_id: String,
    pub label: String,
    pub kind: String,
    /// Label, alias, or framework string that matched the posting text.
    pub matched_phrase: String,
}

/// Collect searchable phrases for a catalog node (label, aliases, framework ref strings).
pub fn skill_search_terms(node: &SkillNode) -> Vec<String> {
    let mut terms = BTreeSet::new();
    let label = node.label.trim();
    if label.len() >= 2 {
        terms.insert(label.to_string());
    }
    for alias in &node.aliases {
        let a = alias.trim();
        if a.len() >= 2 {
            terms.insert(a.to_string());
        }
    }
    collect_json_strings(&node.framework_refs, &mut terms);
    terms.into_iter().collect()
}

fn collect_json_strings(value: &serde_json::Value, out: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::String(s) => {
            let t = s.trim();
            if t.len() >= 3 {
                out.insert(t.to_string());
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_json_strings(v, out);
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_json_strings(v, out);
            }
        }
        _ => {}
    }
}

fn normalize_posting(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_term(term: &str) -> String {
    term.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Match catalog nodes whose label, alias, or framework text appears in `posting_text`.
pub fn match_skills_in_posting(nodes: &[SkillNode], posting_text: &str) -> Vec<SkillPostingMatch> {
    let haystack = normalize_posting(posting_text);
    if haystack.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for node in nodes {
        let mut best: Option<String> = None;
        for term in skill_search_terms(node) {
            let needle = normalize_term(&term);
            if needle.len() < 3 {
                continue;
            }
            if haystack.contains(&needle) {
                best = Some(term);
                break;
            }
        }
        let Some(matched_phrase) = best else {
            continue;
        };
        out.push(SkillPostingMatch {
            skill_id: node.id.clone(),
            label: node.label.clone(),
            kind: node.kind.as_str().to_string(),
            matched_phrase,
        });
    }
    out.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    out
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PostingHintRow {
    pub skill_id: String,
    pub label: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_phrase: Option<String>,
    pub in_posting: bool,
    pub on_draft: bool,
}

pub fn build_posting_hints(
    nodes: &[SkillNode],
    posting_text: &str,
    skill_ids_on_draft: &BTreeSet<String>,
) -> Vec<PostingHintRow> {
    let matches = match_skills_in_posting(nodes, posting_text);
    let matched_ids: BTreeSet<String> = matches.iter().map(|m| m.skill_id.clone()).collect();

    let mut rows = Vec::new();
    for m in &matches {
        rows.push(PostingHintRow {
            skill_id: m.skill_id.clone(),
            label: m.label.clone(),
            kind: m.kind.clone(),
            matched_phrase: Some(m.matched_phrase.clone()),
            in_posting: true,
            on_draft: skill_ids_on_draft.contains(&m.skill_id),
        });
    }

    for node in nodes {
        if matched_ids.contains(&node.id) {
            continue;
        }
        if !skill_ids_on_draft.contains(&node.id) {
            continue;
        }
        rows.push(PostingHintRow {
            skill_id: node.id.clone(),
            label: node.label.clone(),
            kind: node.kind.as_str().to_string(),
            matched_phrase: None,
            in_posting: false,
            on_draft: true,
        });
    }

    rows.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkillKind;
    use serde_json::json;

    fn tag_node(id: &str, label: &str, aliases: Vec<&str>) -> SkillNode {
        SkillNode {
            id: id.into(),
            kind: SkillKind::Tag,
            label: label.into(),
            parent_id: None,
            aliases: aliases.into_iter().map(String::from).collect(),
            pdf_groups: vec![],
            framework_refs: json!({}),
        }
    }

    #[test]
    fn matches_label_and_alias_in_posting() {
        let nodes = vec![tag_node(
            "tag.nist-sp-800-53",
            "NIST SP 800-53",
            vec!["800-53"],
        )];
        let hits = match_skills_in_posting(
            &nodes,
            "Experience with NIST SP 800-53 controls required.",
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].skill_id, "tag.nist-sp-800-53");
    }

    #[test]
    fn build_hints_marks_draft_only() {
        let nodes = vec![
            tag_node("tag.a", "Alpha Standard", vec![]),
            tag_node("tag.b", "Beta Tool", vec![]),
        ];
        let mut on_draft = BTreeSet::new();
        on_draft.insert("tag.b".into());
        let rows = build_posting_hints(&nodes, "We need Alpha Standard experience.", &on_draft);
        assert!(rows.iter().any(|r| r.skill_id == "tag.a" && r.in_posting && !r.on_draft));
        assert!(rows.iter().any(|r| r.skill_id == "tag.b" && !r.in_posting && r.on_draft));
    }
}
