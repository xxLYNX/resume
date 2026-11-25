//! Mirrors browser Showcase approval gating (composition + theme + preview settled).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowcaseApprovalInputs {
    pub saved_composition_json: Option<String>,
    pub saved_theme_id: Option<String>,
    pub live_composition_json: Option<String>,
    pub live_theme_id: Option<String>,
    pub preview_ready_for_saved: bool,
}

pub fn showcase_has_unsaved_edits(input: &ShowcaseApprovalInputs) -> bool {
    let (
        Some(saved_c),
        Some(saved_t),
        Some(live_c),
        Some(live_t),
    ) = (
        input.saved_composition_json.as_ref(),
        input.saved_theme_id.as_ref(),
        input.live_composition_json.as_ref(),
        input.live_theme_id.as_ref(),
    ) else {
        return true;
    };
    saved_c != live_c || saved_t != live_t
}

pub fn showcase_approve_enabled(input: &ShowcaseApprovalInputs) -> bool {
    !showcase_has_unsaved_edits(input) && input.preview_ready_for_saved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_only_change_blocks_approval() {
        let saved_c = r#"{"sections":[]}"#.to_string();
        let input = ShowcaseApprovalInputs {
            saved_composition_json: Some(saved_c.clone()),
            saved_theme_id: Some("classic".into()),
            live_composition_json: Some(saved_c),
            live_theme_id: Some("ats-classic".into()),
            preview_ready_for_saved: true,
        };
        assert!(showcase_has_unsaved_edits(&input));
        assert!(!showcase_approve_enabled(&input));
    }

    #[test]
    fn saved_match_without_preview_keeps_approve_disabled() {
        let json = r#"{"sections":[]}"#.to_string();
        let input = ShowcaseApprovalInputs {
            saved_composition_json: Some(json.clone()),
            saved_theme_id: Some("classic".into()),
            live_composition_json: Some(json),
            live_theme_id: Some("classic".into()),
            preview_ready_for_saved: false,
        };
        assert!(!showcase_has_unsaved_edits(&input));
        assert!(!showcase_approve_enabled(&input));
    }
}