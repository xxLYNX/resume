use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use domain::{render_ledger_tex, ApplicationLedgerRow, Compensation};
use render_pdf::write_tex_and_compile;
use time::OffsetDateTime;

use crate::Store;

const LEDGER_TEX: &str = "jobdesk-applications.tex";

pub fn default_ledger_download_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set; use --output for ledger PDF path")?;
    Ok(PathBuf::from(home).join("Downloads/jobdesk-applications.pdf"))
}

impl Store {
    pub fn list_ledger_rows(&self) -> Result<Vec<ApplicationLedgerRow>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT a.company, a.title, a.phase, a.status,
                    a.employment_type, a.work_mode, a.location, a.compensation_json,
                    a.posting_url, a.updated_at, COUNT(s.id)
             FROM applications a
             LEFT JOIN submissions s ON s.application_id = a.id
             GROUP BY a.id
             ORDER BY a.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let compensation_json: String = row.get(7)?;
            let compensation: Compensation =
                serde_json::from_str(&compensation_json).unwrap_or_default();
            Ok(ApplicationLedgerRow {
                company: row.get(0)?,
                title: row.get(1)?,
                phase: row.get(2)?,
                status: row.get(3)?,
                employment_type: row.get(4)?,
                work_mode: row.get(5)?,
                location: row.get(6)?,
                compensation,
                posting_url: row.get(8)?,
                updated_at: row.get(9)?,
                submission_count: row.get::<_, i64>(10)? as u32,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Build ledger PDF at `dest_pdf` (creates parent dirs). Uses `build_dir` for LaTeX intermediates.
    pub fn export_application_ledger_pdf(
        &self,
        build_dir: &Path,
        dest_pdf: &Path,
    ) -> Result<PathBuf> {
        let rows = self.list_ledger_rows()?;
        let generated_at = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        let tex = render_ledger_tex(&rows, &generated_at);
        std::fs::create_dir_all(build_dir)?;
        if let Some(parent) = dest_pdf.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_tex_and_compile(&tex, build_dir, LEDGER_TEX)?;
        let built = build_dir.join("jobdesk-applications.pdf");
        std::fs::copy(&built, dest_pdf)
            .with_context(|| format!("copy ledger PDF to {}", dest_pdf.display()))?;
        Ok(dest_pdf.to_path_buf())
    }
}
