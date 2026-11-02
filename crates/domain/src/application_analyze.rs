use std::collections::BTreeSet;

use serde::Serialize;

use crate::{
    build_posting_hints, match_skills_in_posting, posting_search_tokens, Composition, PostingHintRow,
    SkillNode,
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillAlignmentStatus {
    Evidenced,
    Partial,
    Gap,
    Extra,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillGapRow {
    pub skill_id: String,
    pub label: String,
    pub kind: String,
    pub status: SkillAlignmentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_phrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompositionGapRow {
    pub element_id: String,
    pub kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveAlignment {
    Ok,
    Warn,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ObjectiveAlignmentNote {
    pub status: ObjectiveAlignment,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ApplicationAnalyzeReport {
    pub application_id: String,
    pub company: String,
    pub title: String,
    pub posting_token_count: usize,
    pub posting_hints: Vec<PostingHintRow>,
    pub skill_gaps: Vec<SkillGapRow>,
    pub skill_orphans: Vec<SkillGapRow>,
    pub composition_gaps: Vec<CompositionGapRow>,
    pub suggestions: Vec<String>,
    pub objective_alignment: ObjectiveAlignmentNote,
}

pub struct AnalyzeCatalogElement {
    pub id: String,
    pub kind: String,
    pub objective_text: Option<String>,
    pub asserted_skill_ids: BTreeSet<String>,
}

pub struct ApplicationAnalyzeInput<'a> {
    pub application_id: &'a str,
    pub company: &'a str,
    pub title: &'a str,
    pub posting_text: &'a str,
    pub composition: &'a Composition,
    pub skill_nodes: &'a [SkillNode],
    pub skill_ids_on_draft: &'a BTreeSet<String>,
    /// All skill ids with at least one asserted link anywhere in catalog.
    pub skill_ids_with_catalog_evidence: &'a BTreeSet<String>,
    pub catalog_elements: &'a [AnalyzeCatalogElement],
    pub pinned_element_ids: &'a BTreeSet<String>,
}

fn hint_status(row: &PostingHintRow, has_catalog_evidence: bool) -> SkillAlignmentStatus {
    if row.in_posting && row.on_draft {
        SkillAlignmentStatus::Evidenced
    } else if row.in_posting && has_catalog_evidence {
        SkillAlignmentStatus::Partial
    } else if row.in_posting {
        SkillAlignmentStatus::Gap
    } else {
        SkillAlignmentStatus::Extra
    }
}

fn objective_alignment(title: &str, objective: Option<&str>) -> ObjectiveAlignmentNote {
    let Some(obj) = objective.map(str::trim).filter(|s| !s.is_empty()) else {
        return ObjectiveAlignmentNote {
            status: ObjectiveAlignment::Unknown,
            detail: "No objective element in draft composition.".into(),
        };
    };
    let title_l = title.to_lowercase();
    let obj_l = obj.to_lowercase();

    const ENGINEERING_TITLE: &[&str] = &[
        "engineer", "developer", "software", "sensor", "platform", "firmware",
    ];
    const ANALYST_OBJECTIVE: &[&str] = &[
        "analyst", "soc ", " soc", "security operations center", "help desk",
    ];

    let title_eng = ENGINEERING_TITLE.iter().any(|t| title_l.contains(t));
    let obj_analyst = ANALYST_OBJECTIVE.iter().any(|t| obj_l.contains(t));

    if title_eng && obj_analyst {
        return ObjectiveAlignmentNote {
            status: ObjectiveAlignment::Warn,
            detail: "Posting title reads engineering-focused but objective mentions analyst/SOC-style roles — consider reframing.".into(),
        };
    }

    ObjectiveAlignmentNote {
        status: ObjectiveAlignment::Ok,
        detail: "No obvious title vs objective role mismatch detected.".into(),
    }
}

pub fn build_application_analyze(input: ApplicationAnalyzeInput<'_>) -> ApplicationAnalyzeReport {
    let posting_hints = build_posting_hints(
        input.skill_nodes,
        input.posting_text,
        input.skill_ids_on_draft,
    );

    let matches = match_skills_in_posting(input.skill_nodes, input.posting_text);
    let matched_skill_ids: BTreeSet<String> = matches.iter().map(|m| m.skill_id.clone()).collect();

    let mut skill_gaps = Vec::new();
    let mut skill_orphans = Vec::new();

    for row in &posting_hints {
        let has_evidence = input
            .skill_ids_with_catalog_evidence
            .contains(&row.skill_id);
        let status = hint_status(row, has_evidence);
        let gap_row = SkillGapRow {
            skill_id: row.skill_id.clone(),
            label: row.label.clone(),
            kind: row.kind.clone(),
            status,
            matched_phrase: row.matched_phrase.clone(),
        };
        match status {
            SkillAlignmentStatus::Gap | SkillAlignmentStatus::Partial => skill_gaps.push(gap_row),
            SkillAlignmentStatus::Extra => skill_orphans.push(gap_row),
            SkillAlignmentStatus::Evidenced => {}
        }
    }

    skill_gaps.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    skill_orphans.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));

    let mut composition_gaps = Vec::new();
    for el in input.catalog_elements {
        if input.pinned_element_ids.contains(&el.id) {
            continue;
        }
        let overlaps: Vec<String> = el
            .asserted_skill_ids
            .iter()
            .filter(|id| matched_skill_ids.contains(*id))
            .cloned()
            .collect();
        if overlaps.is_empty() {
            continue;
        }
        composition_gaps.push(CompositionGapRow {
            element_id: el.id.clone(),
            kind: el.kind.clone(),
            reason: format!(
                "Portfolio entry has evidence matching the posting but is not included in the draft composition ({} skill overlap).",
                overlaps.len()
            ),
        });
    }

    let objective_text = input
        .catalog_elements
        .iter()
        .find(|e| e.kind == "objective" && input.pinned_element_ids.contains(&e.id))
        .and_then(|e| e.objective_text.as_deref());

    let objective_alignment = objective_alignment(input.title, objective_text);

    let mut suggestions = Vec::new();
    if input.posting_text.trim().is_empty() {
        suggestions.push("Paste full job description into posting text for stronger analysis.".into());
    }
    if !skill_gaps.iter().any(|g| g.status == SkillAlignmentStatus::Partial) {
        if skill_gaps.iter().any(|g| g.status == SkillAlignmentStatus::Gap) {
            suggestions.push(
                "Link matching tools/competencies on portfolio entries, then include those entries on the Resume draft.".into(),
            );
        }
    } else {
        suggestions.push(
            "Some posting keywords match catalog evidence but are not on the draft — open Resume and include pinned elements with those tags.".into(),
        );
    }
    for gap in &composition_gaps {
        suggestions.push(format!(
            "Include `{}` in the draft composition — it already supports posting keywords.",
            gap.element_id
        ));
    }
    if objective_alignment.status == ObjectiveAlignment::Warn {
        suggestions.push("Revise objective text (Portfolio → objective element) to match this engineering posting.".into());
    }
    if skill_orphans.len() > 8 {
        suggestions.push(format!(
            "{} skills on draft are not mentioned in posting — trim or refocus for this role.",
            skill_orphans.len()
        ));
    }

    ApplicationAnalyzeReport {
        application_id: input.application_id.to_string(),
        company: input.company.to_string(),
        title: input.title.to_string(),
        posting_token_count: posting_search_tokens(input.posting_text).len(),
        posting_hints,
        skill_gaps,
        skill_orphans,
        composition_gaps,
        suggestions,
        objective_alignment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Composition, SkillKind};
    use serde_json::json;

    fn tag(id: &str, label: &str) -> SkillNode {
        SkillNode {
            id: id.into(),
            kind: SkillKind::Tool,
            label: label.into(),
            parent_id: None,
            aliases: vec![],
            pdf_groups: vec![],
            framework_refs: json!({}),
        }
    }

    #[test]
    fn marks_partial_when_catalog_has_evidence_not_on_draft() {
        let nodes = vec![tag("tool.python", "Python")];
        let mut catalog_evidence = BTreeSet::new();
        catalog_evidence.insert("tool.python".into());
        let on_draft = BTreeSet::new();
        let report = build_application_analyze(ApplicationAnalyzeInput {
            application_id: "a1",
            company: "Co",
            title: "Engineer",
            posting_text: "Must know Python.",
            composition: &Composition { sections: vec![] },
            skill_nodes: &nodes,
            skill_ids_on_draft: &on_draft,
            skill_ids_with_catalog_evidence: &catalog_evidence,
            catalog_elements: &[],
            pinned_element_ids: &BTreeSet::new(),
        });
        assert_eq!(report.skill_gaps.len(), 1);
        assert_eq!(report.skill_gaps[0].status, SkillAlignmentStatus::Partial);
    }
}
