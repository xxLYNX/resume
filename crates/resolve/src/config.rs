use std::collections::BTreeMap;

use theme::ThemeInfo;

/// Per-section limits applied when validating composition resume-line tag picks.
#[derive(Debug, Clone, Default)]
pub struct ResolveConfig {
    /// Section key → max `resume_tag_skill_ids` length (e.g. `projects` → 5).
    pub resume_line_tags_max: BTreeMap<String, usize>,
}

impl ResolveConfig {
    pub fn from_theme(theme: &ThemeInfo) -> Self {
        let mut resume_line_tags_max = BTreeMap::new();
        for (key, sec) in &theme.sections {
            if let Some(max) = sec.resume_line_tags_max {
                resume_line_tags_max.insert(key.clone(), max);
            }
        }
        Self {
            resume_line_tags_max,
        }
    }
}
