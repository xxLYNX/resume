/// Super-status: preparing to apply vs already sent (manifest may exist).
pub const APPLICATION_PHASES: &[&str] = &["draft", "submitted"];

/// Workflow value while `phase = draft` (only this value is valid).
pub const DRAFT_WORKFLOW: &str = "draft";

/// Pipeline labels after at least one submit (`phase = submitted`).
pub const SUBMITTED_WORKFLOWS: &[&str] = &[
    "submitted",
    "screening",
    "interview",
    "offer",
    "rejected",
    "withdrawn",
];

/// Deprecated flat list (draft + all workflows); prefer phase + workflow APIs.
pub const APPLICATION_STATUSES: &[&str] = &[
    "draft", "submitted", "screening", "interview", "offer", "rejected", "withdrawn",
];

pub fn is_valid_phase(phase: &str) -> bool {
    APPLICATION_PHASES.contains(&phase)
}

pub fn is_valid_workflow_for_phase(phase: &str, workflow: &str) -> bool {
    match phase {
        "draft" => workflow == DRAFT_WORKFLOW,
        "submitted" => SUBMITTED_WORKFLOWS.contains(&workflow),
        _ => false,
    }
}

pub fn validate_phase_and_workflow(phase: &str, workflow: &str) -> Result<(), InvalidStatus> {
    if !is_valid_phase(phase) {
        return Err(InvalidStatus(format!("invalid phase `{phase}`")));
    }
    if !is_valid_workflow_for_phase(phase, workflow) {
        return Err(InvalidStatus(format!(
            "workflow `{workflow}` invalid for phase `{phase}`"
        )));
    }
    Ok(())
}

/// Legacy single-field status (pre–phase column).
pub fn is_valid_application_status(status: &str) -> bool {
    status == DRAFT_WORKFLOW || SUBMITTED_WORKFLOWS.contains(&status)
}

pub fn validate_application_status(status: &str) -> Result<(), InvalidStatus> {
    if is_valid_application_status(status) {
        Ok(())
    } else {
        Err(InvalidStatus(status.to_string()))
    }
}

/// Infer phase + workflow from legacy `status` column only.
pub fn phase_and_workflow_from_status(status: &str) -> (&'static str, &'static str) {
    match status {
        DRAFT_WORKFLOW => ("draft", DRAFT_WORKFLOW),
        "submitted" => ("submitted", "submitted"),
        "screening" => ("submitted", "screening"),
        "interview" => ("submitted", "interview"),
        "offer" => ("submitted", "offer"),
        "rejected" => ("submitted", "rejected"),
        "withdrawn" => ("submitted", "withdrawn"),
        _ => ("submitted", "submitted"),
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid application status `{0}`")]
pub struct InvalidStatus(pub String);
