pub mod applications;
pub mod catalog;
mod export;
mod import;
mod migrate;

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

pub use applications::{
    load_manifest_at, CreateApplicationRequest, ApplicationDetail, ApplicationSummary,
    SubmissionDetail, SubmissionSummary, UpdateApplicationRequest, DEFAULT_THEME_ID,
};
pub use catalog::{
    kind_from_str, CreateRevisionRequest, ElementDetail, ElementSummary, RevisionDetail,
    RevisionSummary,
};
pub use export::{export_catalog_to_content, STAGING_CONTENT_DIR};
pub use import::{import_legacy_content, ImportOptions, ImportStats};
pub use migrate::migrate;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
