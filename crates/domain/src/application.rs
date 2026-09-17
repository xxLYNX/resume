/// Application lifecycle statuses (stored as lowercase strings in SQLite).
pub const APPLICATION_STATUSES: &[&str] = &[
    "draft",
    "submitted",
    "screening",
    "interview",
    "offer",
    "rejected",
    "withdrawn",
];

pub fn is_valid_application_status(status: &str) -> bool {
    APPLICATION_STATUSES.contains(&status)
}

pub fn validate_application_status(status: &str) -> Result<(), InvalidStatus> {
    if is_valid_application_status(status) {
        Ok(())
    } else {
        Err(InvalidStatus(status.to_string()))
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid application status `{0}`")]
pub struct InvalidStatus(pub String);
