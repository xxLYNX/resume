pub mod applications;
pub mod catalog;
mod export;
mod import;
mod ledger;
pub mod legacy_inventory;
mod migrate;
pub mod postings;
pub mod presets;
pub mod showcase;
pub mod skills;

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

pub use applications::{
    load_manifest_at, ApplicationDetail, ApplicationSummary, CreateApplicationRequest,
    ListApplicationsQuery, QuickfillRequest, SubmissionDetail, SubmissionSummary,
    UpdateApplicationRequest, DEFAULT_THEME_ID,
};
pub use catalog::{
    kind_from_str, CreateElementRequest, CreateRevisionRequest, ElementDetail, ElementSummary,
    ElementUsageReport, RevisionDetail, RevisionSummary,
};
pub use export::{export_catalog_to_content, STAGING_CONTENT_DIR};
pub use import::{import_legacy_content, ImportOptions, ImportStats};
pub use ledger::default_ledger_download_path;
pub use legacy_inventory::write_legacy_skills_inventory;
pub use migrate::migrate;
pub use showcase::showcase_is_conflict;
pub use postings::{
    normalize_posting_url, PostingDetail, PostingFetchAttemptSummary, PostingInterpretationDetail,
    PostingSnapshotSummary, PostingSummary, PriorReviewedInterpretation,
    UpdatePostingInterpretationRequest,
};
pub use presets::{
    create_preset, delete_preset, get_preset, list_presets, update_preset, CreatePresetRequest,
    PresetDetail, PresetSummary, UpdatePresetRequest,
};
pub use skills::{
    CompetencySuggestion, ElementSkillLinkSummary, ListSkillRelationshipsFilter, SkillLinkRow,
    SkillRelationshipRow,
};

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
