//! Client-side Showcase approval predicates (mirror of `showcase-approval-state.mjs`).
//!
//! The browser enforces these rules before calling approve; the server only approves
//! the persisted draft. Keep behavior in sync via `tests/fixtures/showcase_approval_cases.json`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowcaseUnsavedEditsInput<'a> {
    pub saved_composition: Option<&'a str>,
    pub saved_theme: Option<&'a str>,
    pub live_composition: Option<&'a str>,
    pub live_theme: Option<&'a str>,
}

pub fn showcase_has_unsaved_edits(input: ShowcaseUnsavedEditsInput<'_>) -> bool {
    let (
        Some(saved_c),
        Some(saved_t),
        Some(live_c),
        Some(live_t),
    ) = (
        input.saved_composition,
        input.saved_theme,
        input.live_composition,
        input.live_theme,
    ) else {
        return true;
    };
    saved_c != live_c || saved_t != live_t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowcasePreviewEnablesApproveInput<'a> {
    pub saved_composition: Option<&'a str>,
    pub saved_theme: Option<&'a str>,
    pub snapshot_composition: Option<&'a str>,
    pub snapshot_theme: Option<&'a str>,
    pub live_composition: Option<&'a str>,
    pub live_theme: Option<&'a str>,
    pub request_id: u64,
    pub current_request_id: u64,
}

pub fn showcase_preview_enables_approve(input: ShowcasePreviewEnablesApproveInput<'_>) -> bool {
    if input.request_id != input.current_request_id {
        return false;
    }
    let (
        Some(saved_c),
        Some(saved_t),
        Some(snapshot_c),
        Some(snapshot_t),
        Some(live_c),
        Some(live_t),
    ) = (
        input.saved_composition,
        input.saved_theme,
        input.snapshot_composition,
        input.snapshot_theme,
        input.live_composition,
        input.live_theme,
    ) else {
        return false;
    };
    if snapshot_c != saved_c || snapshot_t != saved_t {
        return false;
    }
    if live_c != saved_c || live_t != saved_t {
        return false;
    }
    true
}

/// Fail closed when shared rules are missing or not yet loaded (missing JS module).
pub fn showcase_preview_approve_enabled(
    rules_module_loaded: bool,
    input: ShowcasePreviewEnablesApproveInput<'_>,
) -> bool {
    if !rules_module_loaded {
        return false;
    }
    showcase_preview_enables_approve(input)
}

pub fn showcase_requires_draft_preview_mode(
    resume_target_type: &str,
    preview_mode_value: &str,
) -> bool {
    if resume_target_type != "showcase" {
        return true;
    }
    preview_mode_value == "draft"
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize)]
    struct FixtureFile {
        strings: HashMap<String, String>,
        cases: Vec<FixtureCase>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureCase {
        name: String,
        op: String,
        #[serde(default)]
        rules_core: Option<String>,
        args: serde_json::Map<String, serde_json::Value>,
        expect: bool,
    }

    fn resolve<'a>(
        value: &'a serde_json::Value,
        strings: &'a HashMap<String, String>,
    ) -> Option<&'a str> {
        match value {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => {
                if strings.contains_key(s) {
                    Some(strings.get(s).expect("key in map").as_str())
                } else {
                    Some(s.as_str())
                }
            }
            _ => None,
        }
    }

    fn u64_arg(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> u64 {
        args.get(key)
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| panic!("case missing u64 arg `{key}`"))
    }

    fn preview_input<'a>(
        args: &'a serde_json::Map<String, serde_json::Value>,
        strings: &'a HashMap<String, String>,
    ) -> ShowcasePreviewEnablesApproveInput<'a> {
        ShowcasePreviewEnablesApproveInput {
            saved_composition: args
                .get("savedComposition")
                .and_then(|v| resolve(v, strings)),
            saved_theme: args.get("savedTheme").and_then(|v| resolve(v, strings)),
            snapshot_composition: args
                .get("snapshotComposition")
                .and_then(|v| resolve(v, strings)),
            snapshot_theme: args
                .get("snapshotTheme")
                .and_then(|v| resolve(v, strings)),
            live_composition: args
                .get("liveComposition")
                .and_then(|v| resolve(v, strings)),
            live_theme: args.get("liveTheme").and_then(|v| resolve(v, strings)),
            request_id: u64_arg(args, "requestId"),
            current_request_id: u64_arg(args, "currentRequestId"),
        }
    }

    #[test]
    fn showcase_approval_cases_match_js_fixture() {
        let raw = include_str!("../tests/fixtures/showcase_approval_cases.json");
        let fixture: FixtureFile =
            serde_json::from_str(raw).expect("parse showcase_approval_cases.json");

        for case in fixture.cases {
            let got = match case.op.as_str() {
                "hasUnsavedEdits" => showcase_has_unsaved_edits(ShowcaseUnsavedEditsInput {
                    saved_composition: case
                        .args
                        .get("savedComposition")
                        .and_then(|v| resolve(v, &fixture.strings)),
                    saved_theme: case
                        .args
                        .get("savedTheme")
                        .and_then(|v| resolve(v, &fixture.strings)),
                    live_composition: case
                        .args
                        .get("liveComposition")
                        .and_then(|v| resolve(v, &fixture.strings)),
                    live_theme: case
                        .args
                        .get("liveTheme")
                        .and_then(|v| resolve(v, &fixture.strings)),
                }),
                "previewEnablesApprove" => {
                    showcase_preview_enables_approve(preview_input(&case.args, &fixture.strings))
                }
                "previewApproveEnabled" => {
                    let loaded = matches!(case.rules_core.as_deref(), Some("present"));
                    showcase_preview_approve_enabled(
                        loaded,
                        preview_input(&case.args, &fixture.strings),
                    )
                }
                "requiresDraftPreviewMode" => {
                    let target = case
                        .args
                        .get("resumeTargetType")
                        .and_then(|v| v.as_str())
                        .expect("resumeTargetType");
                    let mode = case
                        .args
                        .get("previewModeValue")
                        .and_then(|v| v.as_str())
                        .expect("previewModeValue");
                    showcase_requires_draft_preview_mode(target, mode)
                }
                other => panic!("unknown fixture op `{other}` in {}", case.name),
            };
            assert_eq!(got, case.expect, "{}", case.name);
        }
    }
}