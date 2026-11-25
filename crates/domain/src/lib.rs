mod application;
mod application_analyze;
mod application_lifecycle;
mod compensation;
mod composition;
mod dates;
mod framework_refs;
mod greenhouse;
mod greenhouse_content;
mod ledger;
mod location;
mod manifest;
mod posting_fetch;
mod posting_hints;
mod resume_filename;
mod showcase;
mod skills;
mod validation;
mod workday;

use serde::{Deserialize, Serialize};

pub use application::{
    phase_and_workflow_from_status, validate_application_status, validate_phase_and_workflow,
    InvalidStatus, APPLICATION_PHASES, APPLICATION_STATUSES, DRAFT_WORKFLOW, SUBMITTED_WORKFLOWS,
};
pub use application_analyze::{
    build_application_analyze, AnalyzeCatalogElement, ApplicationAnalyzeInput,
    ApplicationAnalyzeReport, CompositionGapRow, ObjectiveAlignment, ObjectiveAlignmentNote,
    SkillAlignmentStatus, SkillGapRow,
};
pub use application_lifecycle::{apply_lifecycle_patch, coerce_phase_and_workflow, LifecycleError};
pub use compensation::{
    annual_equivalent, format_compensation, validate_application_meta, validate_compensation,
    ApplicationMetaError, Compensation, EMPLOYMENT_TYPES, PAY_UNITS, WORK_MODES,
};
pub use composition::{Composition, CompositionItem, CompositionSection};
pub use dates::{
    format_calendar_date, format_date_range, normalize_list_section_document,
    normalize_payload_dates, parse_legacy_date_range, validate_date_range, CalendarDate, DateError,
    DateRange, DateRangeEnd, MonthDisplay, MONTH_ABBR,
};
pub use framework_refs::{merge_framework_refs, merge_framework_refs_into, FrameworkRefsImportRow};
pub use greenhouse::{
    build_greenhouse_provenance, extraction_outcome_usable, greenhouse_api_job_url,
    greenhouse_http_error_outcome, interpret_greenhouse_job_json, is_greenhouse_api_url,
    parse_greenhouse_job_detail_url, GreenhouseJobRef, GREENHOUSE_ADAPTER_ID,
    GREENHOUSE_ADAPTER_VERSION, RETRIEVAL_KIND_GREENHOUSE,
};
pub use greenhouse_content::{
    content_meaningful, decode_entities_capped, greenhouse_content_to_plain,
};
pub use ledger::{render_ledger_tex, ApplicationLedgerRow};
pub use location::WorkLocation;
pub use manifest::{
    compute_content_hash, compute_snapshot_hash, SubmissionManifest, MANIFEST_SCHEMA_VERSION,
};
pub use posting_fetch::{
    detect_source_unavailable, extract_from_html, is_disallowed_destination_ip,
    looks_like_embedded_app_dump, outcome_list_label, parse_extraction_outcome,
    resolve_public_socket_addr, sha256_hex, snapshot_body_for_storage, snapshot_body_for_view,
    validate_final_url_after_redirects, validate_public_fetch_url, ExtractPostingResult,
    FetchedPostingPayload, PostingFetchError, DEFAULT_FETCH_TIMEOUT, DIAG_OUTCOME_PREFIX,
    MAX_FALLBACK_DESCRIPTION_CHARS, MAX_POSTING_BODY_BYTES, MAX_POSTING_REDIRECTS,
    OUTCOME_EXTRACTION_FAILED, OUTCOME_PARTIAL, OUTCOME_RECOGNIZED, OUTCOME_SOURCE_UNAVAILABLE,
    POSTING_EXTRACTOR_VERSION, SNAPSHOT_BODY_PREFIX_BASE64,
};
pub use posting_hints::{
    build_posting_hints, match_skills_in_posting, posting_search_tokens, PostingHintRow,
    SkillPostingMatch,
};
pub use resume_filename::resume_download_filename;
pub use showcase::{
    compute_showcase_snapshot_hash, empty_showcase_composition, sha256_prefixed,
    showcase_composition_is_empty, ShowcaseDraft, ShowcasePublication, SHOWCASE_DEFAULT_THEME_ID,
    SHOWCASE_ID, SHOWCASE_SECTION_KEYS,
};
pub use skills::{
    validate_element_links, validate_pdf_group, validate_skill_id, validate_skill_node,
    ElementSkillLink, SkillError, SkillKind, SkillNode, PDF_SKILL_GROUPS, SKILL_KINDS,
};
pub use validation::{validate_payload, ValidationError, PROJECT_PORTFOLIO_NARRATIVE_KEY};
pub use workday::{
    build_workday_provenance, extract_url_requisition, interpret_workday_cxs_json,
    is_workday_cxs_url, parse_workday_job_detail_url, workday_cxs_job_url,
    workday_http_error_outcome, WorkdayJobRef, RETRIEVAL_KIND_WORKDAY, WORKDAY_ADAPTER_ID,
    WORKDAY_ADAPTER_VERSION,
};

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
    /// Optional PDF section heading (from theme.toml `title`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_title: Option<String>,
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
    Course,
    Certification,
    Award,
}

impl ElementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Objective => "objective",
            Self::Experience => "experience",
            Self::Education => "education",
            Self::Project => "project",
            Self::Course => "course",
            Self::Certification => "certification",
            Self::Award => "award",
        }
    }
}
