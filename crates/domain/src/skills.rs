use serde::{Deserialize, Serialize};

/// Skill axis (what / with / for).
pub const SKILL_KINDS: &[&str] = &["with", "what", "for"];

/// Classic PDF skill line keys in `skills.*` render context.
pub const PDF_SKILL_GROUPS: &[&str] = &["languages", "securityTools", "certifications", "other"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillKind {
    With,
    What,
    For,
}

impl SkillKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::With => "with",
            Self::What => "what",
            Self::For => "for",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "with" => Some(Self::With),
            "what" => Some(Self::What),
            "for" => Some(Self::For),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillNode {
    pub id: String,
    pub kind: SkillKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Which classic PDF groups this label appears under when aggregated.
    #[serde(default, rename = "pdf_groups")]
    pub pdf_groups: Vec<String>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub framework_refs: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionSkillLink {
    pub skill_id: String,
    pub axis: SkillKind,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "asserted".to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("invalid skill id `{0}`")]
    InvalidId(String),
    #[error("invalid skill kind `{0}`")]
    InvalidKind(String),
    #[error("skill `{skill_id}` kind `{actual}` does not match link axis `{axis}`")]
    KindAxisMismatch {
        skill_id: String,
        actual: String,
        axis: String,
    },
    #[error("unknown skill id `{0}`")]
    UnknownSkillId(String),
    #[error("invalid pdf group `{0}`")]
    InvalidPdfGroup(String),
}

pub fn validate_skill_id(id: &str) -> Result<(), SkillError> {
    let id = id.trim();
    if id.is_empty() || id.contains(char::is_whitespace) {
        return Err(SkillError::InvalidId(id.to_string()));
    }
    Ok(())
}

pub fn validate_pdf_group(group: &str) -> Result<(), SkillError> {
    if PDF_SKILL_GROUPS.contains(&group) {
        Ok(())
    } else {
        Err(SkillError::InvalidPdfGroup(group.to_string()))
    }
}

pub fn validate_skill_node(node: &SkillNode) -> Result<(), SkillError> {
    validate_skill_id(&node.id)?;
    if node.label.trim().is_empty() {
        return Err(SkillError::InvalidId("empty label".into()));
    }
    for g in &node.pdf_groups {
        validate_pdf_group(g)?;
    }
    Ok(())
}

pub fn validate_revision_links(
    links: &[RevisionSkillLink],
    node_kind: impl Fn(&str) -> Option<SkillKind>,
) -> Result<(), SkillError> {
    for link in links {
        validate_skill_id(&link.skill_id)?;
        let Some(kind) = node_kind(&link.skill_id) else {
            return Err(SkillError::UnknownSkillId(link.skill_id.clone()));
        };
        if kind != link.axis {
            return Err(SkillError::KindAxisMismatch {
                skill_id: link.skill_id.clone(),
                actual: kind.as_str().into(),
                axis: link.axis.as_str().into(),
            });
        }
        if link.source != "asserted" && link.source != "inferred" {
            return Err(SkillError::InvalidKind(link.source.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_axis_must_match_node() {
        let links = vec![RevisionSkillLink {
            skill_id: "tool.ghidra".into(),
            axis: SkillKind::What,
            source: "asserted".into(),
        }];
        let err = validate_revision_links(&links, |id| {
            if id == "tool.ghidra" {
                Some(SkillKind::With)
            } else {
                None
            }
        })
        .unwrap_err();
        assert!(matches!(err, SkillError::KindAxisMismatch { .. }));
    }
}
