mod parse;
mod slug;

use domain::SkillNode;

pub use parse::{import_from_onet_dir, ToolCompetencyEdge, ToolRow};

/// Minimum O*NET Skills Importance (IM) rating for a competency to count on an occupation.
pub const DEFAULT_COMPETENCY_IMPORTANCE_THRESHOLD: f64 = 3.0;

#[derive(Debug, Clone)]
pub struct OnetImportResult {
    pub tools: Vec<SkillNode>,
    pub competencies: Vec<SkillNode>,
    pub edges: Vec<ToolCompetencyEdge>,
}

pub fn all_skill_nodes(result: &OnetImportResult) -> Vec<SkillNode> {
    let mut out = result.tools.clone();
    out.extend(result.competencies.clone());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onet")
    }

    #[test]
    fn imports_fixture_bundle() {
        let result = import_from_onet_dir(&fixture_dir()).expect("fixture import");
        assert!(result.tools.len() >= 2);
        assert!(result.competencies.len() >= 2);
        assert!(!result.edges.is_empty());
        let ids: std::collections::BTreeSet<_> = result
            .tools
            .iter()
            .map(|n| n.id.clone())
            .collect();
        assert_eq!(ids.len(), result.tools.len(), "tool ids deduped");
    }
}
