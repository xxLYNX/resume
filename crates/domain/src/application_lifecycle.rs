//! Authoritative application phase/workflow transition rules (super-status + pipeline).

use crate::{phase_and_workflow_from_status, validate_phase_and_workflow, DRAFT_WORKFLOW};

#[derive(Debug, Clone, thiserror::Error)]
pub enum LifecycleError {
    #[error("{0}")]
    InvalidTransition(String),
    #[error("invalid phase `{0}`")]
    UnknownPhase(String),
    #[error(transparent)]
    InvalidStatus(#[from] crate::InvalidStatus),
}

/// Apply a partial lifecycle update from HTTP/CLI (phase and/or workflow may be omitted).
///
/// When only `requested_workflow` is set, phase is inferred via [`phase_and_workflow_from_status`]
/// (legacy single-field status compatibility).
pub fn apply_lifecycle_patch(
    current_phase: &str,
    current_workflow: &str,
    requested_phase: Option<&str>,
    requested_workflow: Option<&str>,
    has_submission: bool,
) -> Result<(String, String), LifecycleError> {
    let (phase, workflow) = match (requested_phase, requested_workflow) {
        (None, None) => (current_phase.to_string(), current_workflow.to_string()),
        (Some(p), None) => (p.to_string(), current_workflow.to_string()),
        (None, Some(w)) => {
            let (p, wf) = phase_and_workflow_from_status(w);
            (p.to_string(), wf.to_string())
        }
        (Some(p), Some(w)) => {
            if p == "draft" && w != DRAFT_WORKFLOW {
                return Err(LifecycleError::InvalidTransition(format!(
                    "workflow `{w}` invalid for phase `draft`; only `{DRAFT_WORKFLOW}` is allowed"
                )));
            }
            if p == "submitted" && w == DRAFT_WORKFLOW {
                return Err(LifecycleError::InvalidTransition(
                    "workflow `draft` invalid for phase `submitted`; use a pipeline label".into(),
                ));
            }
            (p.to_string(), w.to_string())
        }
    };

    coerce_phase_and_workflow(&phase, &workflow, has_submission)
}

/// Enforce invariants after phase/workflow are known.
pub fn coerce_phase_and_workflow(
    phase: &str,
    workflow: &str,
    has_submission: bool,
) -> Result<(String, String), LifecycleError> {
    if phase == "draft" {
        if has_submission {
            return Err(LifecycleError::InvalidTransition(
                "cannot return to draft phase after a submission; update pipeline workflow instead"
                    .into(),
            ));
        }
        validate_phase_and_workflow("draft", DRAFT_WORKFLOW)?;
        return Ok(("draft".into(), DRAFT_WORKFLOW.into()));
    }

    if phase == "submitted" {
        if !has_submission {
            return Err(LifecycleError::InvalidTransition(
                "cannot set phase to submitted before the first Submit; use Submit to record what you sent"
                    .into(),
            ));
        }
        let workflow = if workflow == DRAFT_WORKFLOW {
            "submitted".to_string()
        } else {
            workflow.to_string()
        };
        validate_phase_and_workflow("submitted", &workflow)?;
        return Ok(("submitted".into(), workflow));
    }

    Err(LifecycleError::UnknownPhase(phase.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_stays_draft_without_submission() {
        let (p, w) = coerce_phase_and_workflow("draft", "draft", false).unwrap();
        assert_eq!((p.as_str(), w.as_str()), ("draft", "draft"));
    }

    #[test]
    fn cannot_return_to_draft_after_submit() {
        assert!(coerce_phase_and_workflow("draft", "draft", true).is_err());
    }

    #[test]
    fn submitted_phase_requires_manifest() {
        assert!(coerce_phase_and_workflow("submitted", "interview", false).is_err());
    }

    #[test]
    fn pipeline_interview_with_submission() {
        let (p, w) = coerce_phase_and_workflow("submitted", "interview", true).unwrap();
        assert_eq!((p.as_str(), w.as_str()), ("submitted", "interview"));
    }

    #[test]
    fn workflow_only_update_infers_phase() {
        let (p, w) =
            apply_lifecycle_patch("draft", "draft", None, Some("interview"), true).unwrap();
        assert_eq!((p.as_str(), w.as_str()), ("submitted", "interview"));
    }

    #[test]
    fn workflow_only_interview_without_submission_rejected() {
        assert!(apply_lifecycle_patch("draft", "draft", None, Some("interview"), false).is_err());
    }

    #[test]
    fn legacy_mismatch_phase_submitted_zero_submissions_blocks_touch() {
        assert!(
            apply_lifecycle_patch("submitted", "submitted", None, Some("screening"), false)
                .is_err()
        );
    }

    #[test]
    fn explicit_contradictory_phase_workflow_rejected() {
        let err = apply_lifecycle_patch("draft", "draft", Some("draft"), Some("interview"), false)
            .unwrap_err();
        assert!(err.to_string().contains("invalid for phase `draft`"));
    }

    /// Same request must not silently accept contradictory phase/workflow.
    #[test]
    fn explicit_submitted_phase_with_draft_workflow_rejected() {
        let err = apply_lifecycle_patch(
            "submitted",
            "interview",
            Some("submitted"),
            Some("draft"),
            true,
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid for phase `submitted`"));
    }

    /// Omitted workflow: persisted pipeline label is kept (not treated as explicit `draft`).
    #[test]
    fn phase_only_submitted_keeps_existing_pipeline_workflow() {
        let (p, w) =
            apply_lifecycle_patch("submitted", "interview", Some("submitted"), None, true).unwrap();
        assert_eq!((p.as_str(), w.as_str()), ("submitted", "interview"));
    }

    /// Omitted workflow: legacy persisted `draft` workflow normalizes to pipeline `submitted`.
    #[test]
    fn phase_only_submitted_normalizes_persisted_draft_workflow() {
        let (p, w) =
            apply_lifecycle_patch("submitted", "draft", Some("submitted"), None, true).unwrap();
        assert_eq!((p.as_str(), w.as_str()), ("submitted", "submitted"));
    }

    #[test]
    fn coerce_normalizes_submitted_phase_with_draft_workflow_when_both_explicit_in_coerce_only() {
        // `coerce_phase_and_workflow` is used after merge; explicit pairs are rejected earlier.
        let (p, w) = coerce_phase_and_workflow("submitted", "draft", true).unwrap();
        assert_eq!((p.as_str(), w.as_str()), ("submitted", "submitted"));
    }
}
