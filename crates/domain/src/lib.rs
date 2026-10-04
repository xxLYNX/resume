mod application;
mod compensation;
mod dates;
mod ledger;
mod composition;
mod manifest;
mod validation;

use serde::{Deserialize, Serialize};

pub use application::{
    phase_and_workflow_from_status, validate_application_status, validate_phase_and_workflow,
    InvalidStatus, APPLICATION_PHASES, APPLICATION_STATUSES, DRAFT_WORKFLOW,
    SUBMITTED_WORKFLOWS,
};
pub use compensation::{
    annual_equivalent, format_compensation, validate_application_meta, validate_compensation,
    ApplicationMetaError, Compensation, EMPLOYMENT_TYPES, PAY_UNITS, WORK_MODES,
};
pub use composition::{Composition, CompositionItem, CompositionSection};
pub use dates::{
    format_calendar_date, format_date_range, normalize_list_section_document,
    normalize_payload_dates, parse_legacy_date_range, validate_date_range, CalendarDate,
    DateError, DateRange, DateRangeEnd, MonthDisplay, MONTH_ABBR,
};
pub use ledger::{render_ledger_tex, ApplicationLedgerRow};
pub use manifest::{
    compute_content_hash, compute_snapshot_hash, SubmissionManifest, MANIFEST_SCHEMA_VERSION,
};
pub use validation::{validate_payload, ValidationError};

/// Build config: section order + templates. Content comes from catalog resolve (jobdesk).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacyConfig {
    pub sections: Vec<LegacySection>,
    pub output: LegacyOutput,
    pub preamble: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacySection {
    pub name: String,
    pub enabled: bool,
    pub template: String,
    /// File-based test fixtures only. jobdesk resolves content from SQLite.
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacyOutput {
    pub directory: String,
    pub filename: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Header,
    Objective,
    Experience,
    Education,
    Project,
    Skills,
}

impl ElementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Objective => "objective",
            Self::Experience => "experience",
            Self::Education => "education",
            Self::Project => "project",
            Self::Skills => "skills",
        }
    }
}
