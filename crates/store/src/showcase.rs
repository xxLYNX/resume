use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use domain::{
    compute_showcase_snapshot_hash, sha256_prefixed, showcase_composition_is_empty,
    Composition, ShowcaseDraft, ShowcasePublication, SHOWCASE_ID,
};
use domain::ShowcaseConflict;
use rusqlite::params;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

static PDF_TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

use crate::Store;

pub fn showcase_is_conflict(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| cause.is::<ShowcaseConflict>())
}

/// Draft row as read for an approval attempt. The commit rejects the attempt if
/// the theme, composition JSON, timestamp, or current publication pointer changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedShowcaseDraft {
    pub theme_id: String,
    pub composition: Composition,
    pub composition_json: String,
    pub current_publication_id: Option<String>,
    pub updated_at: String,
}

impl CapturedShowcaseDraft {
    pub fn draft(&self) -> ShowcaseDraft {
        ShowcaseDraft {
            theme_id: self.theme_id.clone(),
            composition: self.composition.clone(),
            current_publication_id: self.current_publication_id.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

impl Store {
    pub fn get_showcase(&self) -> Result<ShowcaseDraft> {
        Ok(self.capture_showcase()?.draft())
    }

    pub fn capture_showcase(&self) -> Result<CapturedShowcaseDraft> {
        let conn = self.connection();
        conn.query_row(
            "SELECT theme_id, draft_composition_json, current_publication_id, updated_at
             FROM showcase WHERE id = ?1",
            [SHOWCASE_ID],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("showcase row missing"))
        .and_then(
            |(theme_id, composition_json, current_publication_id, updated_at)| {
                let composition =
                    serde_json::from_str(&composition_json).context("parse showcase draft")?;
                Ok(CapturedShowcaseDraft {
                    theme_id,
                    composition,
                    composition_json,
                    current_publication_id,
                    updated_at,
                })
            },
        )
    }

    /// Replace the draft. Does not change publication rows or the current pointer.
    /// Replaces only the saved showcase theme. Composition and publications stay as they are.
    pub fn apply_showcase_theme(
        &self,
        theme_id: &str,
        expected_updated_at: &str,
    ) -> Result<ShowcaseDraft> {
        let captured = self.capture_showcase()?;
        self.save_showcase_draft_versioned(
            theme_id,
            &captured.composition,
            Some(expected_updated_at),
        )
    }

    pub fn save_showcase_draft(
        &self,
        theme_id: &str,
        composition: &Composition,
    ) -> Result<ShowcaseDraft> {
        self.save_showcase_draft_versioned(theme_id, composition, None)
    }

    /// When `expected_updated_at` is set, the update fails if another save landed first (optimistic concurrency).
    pub fn save_showcase_draft_versioned(
        &self,
        theme_id: &str,
        composition: &Composition,
        expected_updated_at: Option<&str>,
    ) -> Result<ShowcaseDraft> {
        let theme_id = theme_id.trim();
        if theme_id.is_empty() {
            bail!("showcase theme id is required");
        }
        let draft_json = serde_json::to_string(composition)?;
        let now = now_rfc3339();
        let n = if let Some(expected) = expected_updated_at {
            self.connection().execute(
                "UPDATE showcase
                 SET theme_id = ?1, draft_composition_json = ?2, updated_at = ?3
                 WHERE id = ?4 AND updated_at = ?5",
                params![theme_id, draft_json, now, SHOWCASE_ID, expected],
            )?
        } else {
            self.connection().execute(
                "UPDATE showcase
                 SET theme_id = ?1, draft_composition_json = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![theme_id, draft_json, now, SHOWCASE_ID],
            )?
        };
        if n == 0 {
            if expected_updated_at.is_some() {
                return Err(ShowcaseConflict).context(
                    "showcase draft was saved again — reload the draft and merge your edits",
                );
            }
            bail!("showcase row missing");
        }
        self.get_showcase()
    }

    pub fn list_showcase_publications(&self) -> Result<Vec<ShowcasePublication>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT id, created_at, theme_id, theme_files_hash, composition_json, resolved_json,
                    snapshot_hash, pdf_relpath, pdf_sha256
             FROM showcase_publications
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(publication_from_row(row?)?);
        }
        Ok(out)
    }

    pub fn get_showcase_publication(&self, id: &str) -> Result<Option<ShowcasePublication>> {
        let conn = self.connection();
        let row = conn
            .query_row(
                "SELECT id, created_at, theme_id, theme_files_hash, composition_json, resolved_json,
                        snapshot_hash, pdf_relpath, pdf_sha256
                 FROM showcase_publications WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                },
            )
            .optional()?;
        row.map(publication_from_row).transpose()
    }

    /// Record `captured` as a publication. The draft row must still match `captured`
    /// inside the same transaction that inserts the row and moves the pointer.
    /// An identical `snapshot_hash` on the current publication returns that row.
    pub fn approve_showcase_snapshot(
        &self,
        workspace: &Path,
        captured: &CapturedShowcaseDraft,
        resolved: &BTreeMap<String, Value>,
        pdf_bytes: &[u8],
        theme_files_hash: &str,
        enabled_sections: &[String],
    ) -> Result<ShowcasePublication> {
        if showcase_composition_is_empty(&captured.composition) {
            bail!("showcase draft has no pinned elements — publication was not created");
        }
        if pdf_bytes.is_empty() {
            bail!("approved showcase PDF is empty — publication was not created");
        }
        let snapshot_hash = compute_showcase_snapshot_hash(
            &captured.theme_id,
            theme_files_hash,
            enabled_sections,
            &captured.composition,
            resolved,
        );
        if self.captured_draft_unchanged(captured)? {
            if let Some(existing) = self.current_publication_if_same_hash(&snapshot_hash)? {
                if captured.current_publication_id.as_deref() == Some(existing.id.as_str()) {
                    verify_publication_pdf(workspace, &existing)?;
                    return Ok(existing);
                }
            }
        }

        let pdf_sha256 = sha256_prefixed(pdf_bytes);
        let id = Uuid::new_v4().to_string();
        let pdf_relpath = format!("artifacts/showcase/{id}/resume.pdf");
        let dest = workspace.join(&pdf_relpath);
        let pub_dir = dest
            .parent()
            .context("showcase PDF path has no parent")?
            .to_path_buf();
        if let Err(err) = install_frozen_pdf(&dest, pdf_bytes) {
            let _ = fs::remove_dir_all(&pub_dir);
            return Err(err).context(
                "showcase PDF was not installed — publication was not created and the pointer is unchanged",
            );
        }
        if let Err(err) = sync_dir_after_pdf_install(&dest, &pub_dir) {
            return Err(err).context(
                "showcase pointer is unchanged; the new PDF may already have been installed and no publication row was written",
            );
        }

        let created_at = now_rfc3339();
        let resolved_json = serde_json::to_string(resolved)?;
        let recorded = (|| -> Result<Option<String>> {
            let tx = self.connection().unchecked_transaction()?;
            let (theme_id, composition_json, updated_at, current_publication_id): (
                String,
                String,
                String,
                Option<String>,
            ) = tx.query_row(
                "SELECT theme_id, draft_composition_json, updated_at, current_publication_id
                 FROM showcase WHERE id = ?1",
                [SHOWCASE_ID],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            if theme_id != captured.theme_id
                || composition_json != captured.composition_json
                || updated_at != captured.updated_at
                || current_publication_id != captured.current_publication_id
            {
                return Err(ShowcaseConflict).context(
                    "showcase draft or publication pointer changed during approval — publication was not created",
                );
            }
            let current_hash: Option<String> = tx
                .query_row(
                    "SELECT p.snapshot_hash FROM showcase s
                     JOIN showcase_publications p ON p.id = s.current_publication_id
                     WHERE s.id = ?1",
                    [SHOWCASE_ID],
                    |row| row.get(0),
                )
                .optional()?;
            if current_hash.as_deref() == Some(snapshot_hash.as_str()) {
                tx.rollback()?;
                return Ok(None);
            }
            tx.execute(
                "INSERT INTO showcase_publications (
                    id, created_at, theme_id, theme_files_hash, composition_json, resolved_json,
                    snapshot_hash, pdf_relpath, pdf_sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    created_at,
                    captured.theme_id,
                    theme_files_hash,
                    captured.composition_json,
                    resolved_json,
                    snapshot_hash,
                    pdf_relpath,
                    pdf_sha256,
                ],
            )?;
            let n = tx.execute(
                "UPDATE showcase SET current_publication_id = ?1, updated_at = ?2 WHERE id = ?3",
                params![id, created_at, SHOWCASE_ID],
            )?;
            if n != 1 {
                bail!("showcase row missing while recording publication");
            }
            tx.commit()?;
            Ok(Some(id))
        })();
        match recorded {
            Ok(Some(new_id)) => self
                .get_showcase_publication(&new_id)?
                .context("showcase publication missing after insert"),
            Ok(None) => {
                let _ = fs::remove_dir_all(&pub_dir);
                let existing = self
                    .current_publication_if_same_hash(&snapshot_hash)?
                    .context("matching showcase publication missing after duplicate check")?;
                verify_publication_pdf(workspace, &existing)?;
                Ok(existing)
            }
            Err(err) => {
                let _ = fs::remove_dir_all(&pub_dir);
                Err(err).context(
                    "showcase publication was not recorded; the new PDF artifact was removed and the previous pointer is unchanged",
                )
            }
        }
    }

    /// Point the current publication at an older row after the PDF bytes match the stored checksum.
    pub fn restore_showcase_pointer(
        &self,
        workspace: &Path,
        publication_id: &str,
    ) -> Result<ShowcaseDraft> {
        let publication = self
            .get_showcase_publication(publication_id)?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown showcase publication `{publication_id}` — pointer not changed"
                )
            })?;
        verify_publication_pdf(workspace, &publication)
            .with_context(|| format!("showcase publication `{publication_id}` was not restored"))?;
        let now = now_rfc3339();
        self.connection().execute(
            "UPDATE showcase SET current_publication_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![publication_id, now, SHOWCASE_ID],
        )?;
        self.get_showcase()
    }

    fn captured_draft_unchanged(&self, captured: &CapturedShowcaseDraft) -> Result<bool> {
        let current = self.capture_showcase()?;
        Ok(current.theme_id == captured.theme_id
            && current.composition_json == captured.composition_json
            && current.updated_at == captured.updated_at
            && current.current_publication_id == captured.current_publication_id)
    }

    fn current_publication_if_same_hash(
        &self,
        snapshot_hash: &str,
    ) -> Result<Option<ShowcasePublication>> {
        let current = self.get_showcase()?.current_publication_id;
        let Some(id) = current else {
            return Ok(None);
        };
        let Some(publication) = self.get_showcase_publication(&id)? else {
            return Ok(None);
        };
        if publication.snapshot_hash == snapshot_hash {
            Ok(Some(publication))
        } else {
            Ok(None)
        }
    }
}

fn publication_from_row(
    row: (
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        String,
    ),
) -> Result<ShowcasePublication> {
    let (
        id,
        created_at,
        theme_id,
        theme_files_hash,
        composition_json,
        resolved_json,
        snapshot_hash,
        pdf_relpath,
        pdf_sha256,
    ) = row;
    Ok(ShowcasePublication {
        id,
        created_at,
        theme_id,
        theme_files_hash,
        composition: serde_json::from_str(&composition_json)
            .context("parse publication composition")?,
        resolved: serde_json::from_str(&resolved_json).context("parse publication resolved")?,
        snapshot_hash,
        pdf_relpath,
        pdf_sha256,
    })
}

fn verify_publication_pdf(workspace: &Path, publication: &ShowcasePublication) -> Result<()> {
    let path = workspace.join(&publication.pdf_relpath);
    let bytes = fs::read(&path).with_context(|| {
        format!(
            "showcase PDF {} is missing — pointer not changed",
            path.display()
        )
    })?;
    let actual = sha256_prefixed(&bytes);
    if actual != publication.pdf_sha256 {
        bail!(
            "showcase PDF {} checksum mismatch — pointer not changed",
            path.display()
        );
    }
    Ok(())
}

fn install_frozen_pdf(dest: &Path, bytes: &[u8]) -> Result<()> {
    let dir = dest.parent().context("showcase PDF path has no parent")?;
    fs::create_dir_all(dir)?;
    // Persist the new publication directory entry before the PDF is renamed into it.
    sync_created_directory_parents(dir)?;
    let tmp = unique_temp_path(dir);
    let write_result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .with_context(|| format!("create {}", tmp.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        file.sync_all()
            .with_context(|| format!("sync {}", tmp.display()))?;
        drop(file);
        fs::rename(&tmp, dest).with_context(|| {
            format!(
                "install {} — publication was not created and the pointer is unchanged",
                dest.display()
            )
        })?;
        Ok(())
    })();
    if let Err(err) = write_result {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    Ok(())
}

fn sync_created_directory_parents(dir: &Path) -> Result<()> {
    let mut current = dir
        .parent()
        .context("showcase publication directory has no parent")?
        .to_path_buf();
    for _ in 0..3 {
        sync_directory_before_rename(&current)?;
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }
    Ok(())
}

fn sync_directory_before_rename(dir: &Path) -> Result<()> {
    let synced = (|| -> Result<()> {
        let file = fs::File::open(dir)?;
        file.sync_all()?;
        Ok(())
    })();
    synced.with_context(|| {
        format!(
            "sync directory {} before installing the PDF — publication was not created and the pointer is unchanged",
            dir.display()
        )
    })
}

fn sync_dir_after_pdf_install(pdf_path: &Path, dir: &Path) -> Result<()> {
    let synced = (|| -> Result<()> {
        let file = fs::File::open(dir)?;
        file.sync_all()?;
        Ok(())
    })();
    synced.with_context(|| {
        format!(
            "installed {} but syncing directory {} failed — the new PDF may already have been installed",
            pdf_path.display(),
            dir.display()
        )
    })
}

fn unique_temp_path(dir: &Path) -> PathBuf {
    let seq = PDF_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    dir.join(format!(
        ".resume.pdf.{}.{}.{}.tmp",
        std::process::id(),
        nanos,
        seq
    ))
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::CreateElementRequest;
    use domain::{
        empty_showcase_composition, Composition, CompositionItem, CompositionSection,
        SHOWCASE_DEFAULT_THEME_ID, SHOWCASE_SECTION_KEYS,
    };
    use serde_json::json;

    fn open_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("jobdesk.db")).unwrap();
        (dir, store)
    }

    fn approve(
        store: &Store,
        workspace: &Path,
        resolved: &BTreeMap<String, Value>,
        pdf: &[u8],
    ) -> Result<ShowcasePublication> {
        let captured = store.capture_showcase()?;
        store.approve_showcase_snapshot(
            workspace,
            &captured,
            resolved,
            pdf,
            "sha256:theme",
            &["header".to_string(), "experience".to_string()],
        )
    }

    fn header_item(element_id: &str) -> Composition {
        Composition {
            sections: vec![CompositionSection {
                section_key: "header".into(),
                items: vec![CompositionItem {
                    element_id: element_id.into(),
                    revision_id: "rev".into(),
                    resume_tag_skill_ids: Vec::new(),
                }],
            }],
        }
    }

    fn pin(element_id: &str, revision_id: &str) -> Composition {
        let mut composition = empty_showcase_composition();
        for section in &mut composition.sections {
            if section.section_key == "experience" {
                section.items.push(CompositionItem {
                    element_id: element_id.to_string(),
                    revision_id: revision_id.to_string(),
                    resume_tag_skill_ids: Vec::new(),
                });
            }
        }
        composition
    }

    #[test]
    fn fresh_showcase_is_empty_and_has_no_publication() {
        let (_dir, store) = open_store();
        let draft = store.get_showcase().unwrap();
        assert_eq!(draft.theme_id, SHOWCASE_DEFAULT_THEME_ID);
        assert!(draft.current_publication_id.is_none());
        assert_eq!(
            draft.composition.sections.len(),
            SHOWCASE_SECTION_KEYS.len()
        );
        assert!(draft
            .composition
            .sections
            .iter()
            .all(|section| section.items.is_empty()));
        assert!(store.list_showcase_publications().unwrap().is_empty());
    }

    #[test]
    fn upgrade_from_authentic_v17_keeps_portfolio_records() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("jobdesk.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(
            "
            CREATE TABLE elements (
                id TEXT PRIMARY KEY NOT NULL,
                kind TEXT NOT NULL,
                created_at TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                parent_element_id TEXT REFERENCES elements(id)
            );
            CREATE TABLE element_revisions (
                id TEXT PRIMARY KEY NOT NULL,
                element_id TEXT NOT NULL REFERENCES elements(id),
                rev_number INTEGER NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(element_id, rev_number)
            );
            CREATE TABLE applications (
                id TEXT PRIMARY KEY NOT NULL,
                posting_url TEXT,
                company TEXT NOT NULL,
                title TEXT NOT NULL,
                source TEXT,
                status TEXT NOT NULL,
                tags_json TEXT NOT NULL DEFAULT '[]',
                draft_composition_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                phase TEXT NOT NULL DEFAULT 'draft',
                employment_type TEXT,
                work_mode TEXT,
                location TEXT,
                compensation_json TEXT NOT NULL DEFAULT '{}',
                posting_text TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE submissions (
                id TEXT PRIMARY KEY NOT NULL,
                application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
                manifest_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                pdf_path TEXT,
                is_canonical INTEGER NOT NULL DEFAULT 0,
                submitted_at TEXT NOT NULL
            );
            CREATE TABLE presets (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                theme_id TEXT NOT NULL DEFAULT 'classic',
                composition_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE schema_meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            ",
        )
        .unwrap();
        let payload = r#"{"name":"Kept","technologies":["Rust"]}"#;
        let app_draft = r#"{"sections":[{"section_key":"header","items":[{"element_id":"project.keep","revision_id":"rev-keep","resume_tag_skill_ids":[]}]}]}"#;
        let preset_draft = r#"{"sections":[{"section_key":"experience","items":[]}]}"#;
        conn.execute(
            "INSERT INTO elements (id, kind, created_at, sort_order, parent_element_id)
             VALUES ('project.keep', 'project', '2020-01-01T00:00:00Z', 0, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO element_revisions (id, element_id, rev_number, payload_json, created_at)
             VALUES ('rev-keep', 'project.keep', 1, ?1, '2020-01-01T00:00:00Z')",
            [payload],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO applications (
                id, company, title, posting_text, source, phase, status, tags_json,
                employment_type, work_mode, location, compensation_json,
                draft_composition_json, created_at, updated_at
             ) VALUES (
                'app-1', 'Kept Co', 'Kept Role', 'posting body stays', 'board', 'draft', 'draft', '[\"rust\"]',
                'full-time', 'remote', 'Austin', '{\"currency\":\"USD\"}',
                ?1, '2020-01-01T00:00:00Z', '2020-01-02T00:00:00Z'
             )",
            [app_draft],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO presets (id, name, theme_id, composition_json, created_at, updated_at)
             VALUES ('preset-1', 'Kept Preset', 'classic', ?1, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
            [preset_draft],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO submissions (
                id, application_id, manifest_path, content_hash, pdf_path, is_canonical, submitted_at
             ) VALUES (
                'sub-1', 'app-1', 'artifacts/sub-1/manifest.json', 'sha256:kept-submission',
                'artifacts/sub-1/resume.pdf', 1, '2020-02-01T00:00:00Z'
             )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO schema_meta (key, value) VALUES ('schema_version', '17')",
            [],
        )
        .unwrap();
        drop(conn);

        let store = Store::open(&db_path).unwrap();
        let version: String = store
            .connection()
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, "19");
        let app = store.get_application("app-1").unwrap().unwrap();
        assert_eq!(app.company, "Kept Co");
        assert_eq!(app.title, "Kept Role");
        assert_eq!(app.posting_text, "posting body stays");
        assert_eq!(app.phase, "draft");
        assert_eq!(app.status, "draft");
        assert_eq!(app.location.as_deref(), Some("Austin"));
        assert_eq!(
            app.draft_composition.sections[0].items[0].element_id,
            "project.keep"
        );
        assert_eq!(app.submissions.len(), 1);
        assert_eq!(app.submissions[0].content_hash, "sha256:kept-submission");
        let preset = store.get_preset("preset-1").unwrap().unwrap();
        assert_eq!(preset.name, "Kept Preset");
        assert_eq!(preset.composition.sections[0].section_key, "experience");
        let element = store.get_element("project.keep").unwrap().unwrap();
        assert_eq!(element.latest.unwrap().payload["technologies"][0], "Rust");
        let draft = store.get_showcase().unwrap();
        assert!(draft.current_publication_id.is_none());
        assert!(draft
            .composition
            .sections
            .iter()
            .all(|section| section.items.is_empty()));
        assert!(store.list_showcase_publications().unwrap().is_empty());
    }

    #[test]
    fn approval_is_immutable_when_catalog_and_draft_change() {
        let (dir, store) = open_store();
        let element = store
            .create_element(&CreateElementRequest {
                id: "experience.acme".into(),
                kind: "experience".into(),
                payload: json!({"company": "Before", "position": "Eng"}),
                parent_element_id: None,
            })
            .unwrap();
        let revision_id = element.revisions[0].id.clone();
        store
            .save_showcase_draft("classic", &pin("experience.acme", &revision_id))
            .unwrap();
        let resolved = BTreeMap::from([("experience".to_string(), json!({"company": "Before"}))]);
        let pdf = b"%PDF-1.4 frozen-before";
        let publication = approve(&store, dir.path(), &resolved, pdf).unwrap();
        let frozen_hash = publication.snapshot_hash.clone();
        let frozen_resolved = publication.resolved.clone();
        let pdf_bytes = fs::read(dir.path().join(&publication.pdf_relpath)).unwrap();
        assert_eq!(pdf_bytes, pdf);
        assert_eq!(publication.pdf_sha256, sha256_prefixed(pdf));

        store
            .create_revision(
                "experience.acme",
                json!({"company": "After", "position": "Eng"}),
            )
            .unwrap();
        let empty = empty_showcase_composition();
        store.save_showcase_draft("ats-classic", &empty).unwrap();

        let stored = store
            .get_showcase_publication(&publication.id)
            .unwrap()
            .unwrap();
        assert_eq!(stored.resolved, frozen_resolved);
        assert_eq!(stored.snapshot_hash, frozen_hash);
        assert_eq!(stored.theme_id, "classic");
        assert!(stored.composition.references_element("experience.acme"));
        assert_eq!(fs::read(dir.path().join(&stored.pdf_relpath)).unwrap(), pdf);

        let draft = store.get_showcase().unwrap();
        assert_eq!(
            draft.current_publication_id.as_deref(),
            Some(publication.id.as_str())
        );
        assert_eq!(draft.theme_id, "ats-classic");
        assert!(!draft.composition.references_element("experience.acme"));
    }

    #[test]
    fn restore_moves_the_pointer_and_keeps_history_and_draft() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.first"))
            .unwrap();
        let first = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-1").unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.second"))
            .unwrap();
        let second = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-2").unwrap();
        let draft_before = store.get_showcase().unwrap();
        assert_eq!(
            draft_before.current_publication_id.as_deref(),
            Some(second.id.as_str())
        );

        let restored = store
            .restore_showcase_pointer(dir.path(), &first.id)
            .unwrap();
        assert_eq!(
            restored.current_publication_id.as_deref(),
            Some(first.id.as_str())
        );
        assert_eq!(restored.composition, draft_before.composition);
        assert_eq!(restored.theme_id, draft_before.theme_id);
        let history = store.list_showcase_publications().unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(
            fs::read(dir.path().join(&first.pdf_relpath)).unwrap(),
            b"%PDF-1"
        );
        assert_eq!(
            fs::read(dir.path().join(&second.pdf_relpath)).unwrap(),
            b"%PDF-2"
        );
        let err = store
            .restore_showcase_pointer(dir.path(), "missing")
            .unwrap_err();
        assert!(format!("{err}").contains("missing"));
        assert_eq!(
            store
                .get_showcase()
                .unwrap()
                .current_publication_id
                .as_deref(),
            Some(first.id.as_str())
        );
    }

    #[test]
    fn draft_pin_blocks_element_delete_and_publication_does_not() {
        let (dir, store) = open_store();
        let element = store
            .create_element(&CreateElementRequest {
                id: "experience.acme".into(),
                kind: "experience".into(),
                payload: json!({"company": "Acme"}),
                parent_element_id: None,
            })
            .unwrap();
        let revision_id = element.revisions[0].id.clone();
        store
            .save_showcase_draft("classic", &pin("experience.acme", &revision_id))
            .unwrap();
        let err = store
            .delete_element_if_unused("experience.acme")
            .unwrap_err();
        assert!(format!("{err}").contains("showcase draft"), "{err}");

        let resolved = BTreeMap::from([("experience".into(), json!({"company": "Acme"}))]);
        approve(&store, dir.path(), &resolved, b"%PDF-keep").unwrap();
        store
            .save_showcase_draft("classic", &empty_showcase_composition())
            .unwrap();
        store.delete_element_if_unused("experience.acme").unwrap();
        let publication = &store.list_showcase_publications().unwrap()[0];
        assert_eq!(
            publication.resolved.get("experience").unwrap()["company"],
            "Acme"
        );
        assert!(store.get_element("experience.acme").unwrap().is_none());
    }

    #[test]
    fn failed_insert_removes_the_pdf_and_leaves_the_pointer() {
        let (dir, store) = open_store();
        store
            .connection()
            .execute_batch(
                "CREATE TRIGGER showcase_fail_insert BEFORE INSERT ON showcase_publications
                 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
            )
            .unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.orphan"))
            .unwrap();
        let err = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-orphan").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("not recorded") || msg.contains("injected"),
            "{msg}"
        );
        assert!(store.list_showcase_publications().unwrap().is_empty());
        assert!(store
            .get_showcase()
            .unwrap()
            .current_publication_id
            .is_none());
        let artifact_root = dir.path().join("artifacts/showcase");
        if artifact_root.exists() {
            for entry in fs::read_dir(&artifact_root).unwrap() {
                let path = entry.unwrap().path();
                assert!(
                    !path.join("resume.pdf").exists(),
                    "orphan PDF left at {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn empty_pdf_does_not_create_a_publication() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.empty-pdf"))
            .unwrap();
        let err = approve(&store, dir.path(), &BTreeMap::new(), b"").unwrap_err();
        assert!(format!("{err}").contains("empty"));
        assert!(store.list_showcase_publications().unwrap().is_empty());
        assert!(!dir.path().join("artifacts/showcase").exists());
    }

    #[test]
    fn empty_composition_is_rejected_before_a_publication() {
        let (dir, store) = open_store();
        let captured = store.capture_showcase().unwrap();
        let err = store
            .approve_showcase_snapshot(
                dir.path(),
                &captured,
                &BTreeMap::new(),
                b"%PDF",
                "sha256:theme",
                &["header".into()],
            )
            .unwrap_err();
        assert!(format!("{err}").contains("no pinned"));
        assert!(store.list_showcase_publications().unwrap().is_empty());
    }

    #[test]
    fn identical_approval_returns_the_current_publication() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.same"))
            .unwrap();
        let first = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-same").unwrap();
        let second = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-same").unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(store.list_showcase_publications().unwrap().len(), 1);
    }

    #[test]
    fn draft_change_between_capture_and_commit_is_a_conflict() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.before"))
            .unwrap();
        let captured = store.capture_showcase().unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.after"))
            .unwrap();
        let err = store
            .approve_showcase_snapshot(
                dir.path(),
                &captured,
                &BTreeMap::new(),
                b"%PDF-stale",
                "sha256:theme",
                &["header".into(), "experience".into()],
            )
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(showcase_is_conflict(&err), "{msg}");
        assert!(msg.contains("changed"), "{msg}");
        assert!(store.list_showcase_publications().unwrap().is_empty());
        let artifact_root = dir.path().join("artifacts/showcase");
        if artifact_root.exists() {
            for entry in fs::read_dir(&artifact_root).unwrap() {
                assert!(!entry.unwrap().path().join("resume.pdf").exists());
            }
        }
    }

    #[test]
    fn restore_refuses_a_missing_or_mismatched_pdf() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.restore"))
            .unwrap();
        let publication = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-restore").unwrap();
        let path = dir.path().join(&publication.pdf_relpath);
        fs::remove_file(&path).unwrap();
        let missing = store
            .restore_showcase_pointer(dir.path(), &publication.id)
            .unwrap_err();
        assert!(format!("{missing:#}").contains("missing"), "{missing:#}");
        assert_eq!(
            store
                .get_showcase()
                .unwrap()
                .current_publication_id
                .as_deref(),
            Some(publication.id.as_str())
        );
        fs::write(&path, b"%PDF-tampered").unwrap();
        let mismatched = store
            .restore_showcase_pointer(dir.path(), &publication.id)
            .unwrap_err();
        assert!(
            format!("{mismatched:#}").contains("checksum"),
            "{mismatched:#}"
        );
        assert_eq!(
            store
                .get_showcase()
                .unwrap()
                .current_publication_id
                .as_deref(),
            Some(publication.id.as_str())
        );
    }

    #[test]
    fn sql_failure_keeps_the_previous_pdf_and_unrelated_artifacts() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.first"))
            .unwrap();
        let first = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-first").unwrap();
        let first_bytes = fs::read(dir.path().join(&first.pdf_relpath)).unwrap();
        let sentinel = dir.path().join("artifacts/showcase/keep-me");
        fs::create_dir_all(&sentinel).unwrap();
        fs::write(sentinel.join("resume.pdf"), b"%PDF-keep").unwrap();
        store
            .connection()
            .execute_batch(
                "CREATE TRIGGER showcase_fail_insert BEFORE INSERT ON showcase_publications
                 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
            )
            .unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.second"))
            .unwrap();
        let err = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-second").unwrap_err();
        assert!(
            format!("{err:#}").contains("injected") || format!("{err:#}").contains("not recorded")
        );
        assert_eq!(store.list_showcase_publications().unwrap().len(), 1);
        assert_eq!(
            store
                .get_showcase()
                .unwrap()
                .current_publication_id
                .as_deref(),
            Some(first.id.as_str())
        );
        assert_eq!(
            fs::read(dir.path().join(&first.pdf_relpath)).unwrap(),
            first_bytes
        );
        assert_eq!(fs::read(sentinel.join("resume.pdf")).unwrap(), b"%PDF-keep");
    }

    #[test]
    fn file_write_failure_leaves_the_previous_pointer() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.installed"))
            .unwrap();
        let first = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-installed").unwrap();
        let root = dir.path().join("artifacts/showcase");
        let mut perms = fs::metadata(&root).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&root, perms).unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.blocked"))
            .unwrap();
        let err = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-blocked").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("not installed") || msg.contains("Permission") || msg.contains("pointer"),
            "{msg}"
        );
        assert_eq!(
            store
                .get_showcase()
                .unwrap()
                .current_publication_id
                .as_deref(),
            Some(first.id.as_str())
        );
        assert_eq!(store.list_showcase_publications().unwrap().len(), 1);
        let mut perms = fs::metadata(&root).unwrap().permissions();
        perms.set_readonly(false);
        fs::set_permissions(&root, perms).unwrap();
    }

    #[test]
    fn a_later_approval_does_not_delete_an_unreferenced_artifact() {
        let (dir, store) = open_store();
        let orphan = dir.path().join("artifacts/showcase/interrupted");
        fs::create_dir_all(&orphan).unwrap();
        fs::write(orphan.join("resume.pdf"), b"%PDF-orphan-valid").unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.live"))
            .unwrap();
        approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-live").unwrap();
        assert_eq!(
            fs::read(orphan.join("resume.pdf")).unwrap(),
            b"%PDF-orphan-valid"
        );
    }

    #[test]
    fn pdf_temp_names_are_unique() {
        let dir = tempfile::tempdir().unwrap();
        let first = unique_temp_path(dir.path());
        let second = unique_temp_path(dir.path());
        assert_ne!(first, second);
        assert!(first
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains(&std::process::id().to_string()));
        assert_ne!(
            first.file_name().unwrap().to_str().unwrap(),
            format!(".resume.pdf.{}.tmp", std::process::id())
        );
    }

    #[test]
    fn restore_between_capture_and_commit_keeps_the_restored_pointer() {
        let (dir, store) = open_store();
        store
            .save_showcase_draft("classic", &header_item("header.first"))
            .unwrap();
        let first = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-1").unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.second"))
            .unwrap();
        let second = approve(&store, dir.path(), &BTreeMap::new(), b"%PDF-2").unwrap();
        let captured = store.capture_showcase().unwrap();
        assert_eq!(
            captured.current_publication_id.as_deref(),
            Some(second.id.as_str())
        );
        store
            .restore_showcase_pointer(dir.path(), &first.id)
            .unwrap();
        let publications_before = store.list_showcase_publications().unwrap().len();
        let err = store
            .approve_showcase_snapshot(
                dir.path(),
                &captured,
                &BTreeMap::new(),
                b"%PDF-stale",
                "sha256:theme",
                &["header".into(), "experience".into()],
            )
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("changed"), "{msg}");
        assert_eq!(
            store
                .get_showcase()
                .unwrap()
                .current_publication_id
                .as_deref(),
            Some(first.id.as_str())
        );
        assert_eq!(
            store.list_showcase_publications().unwrap().len(),
            publications_before
        );
        assert!(store
            .get_showcase_publication(&second.id)
            .unwrap()
            .is_some());
    }

    #[test]
    fn directory_sync_before_rename_does_not_claim_the_pdf_was_installed() {
        let err = sync_directory_before_rename(Path::new("/tmp/jobdesk-missing-showcase-parent"))
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("before installing"), "{msg}");
        assert!(msg.contains("pointer is unchanged"), "{msg}");
        assert!(!msg.contains("may already have been installed"), "{msg}");
    }

    #[test]
    fn directory_sync_failure_after_install_names_the_installed_file() {
        let err = sync_dir_after_pdf_install(
            Path::new("/tmp/resume.pdf"),
            Path::new("/tmp/jobdesk-missing-showcase-dir"),
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("may already have been installed"), "{msg}");
        assert!(!msg.contains("original left unchanged"), "{msg}");
    }

    #[test]
    fn concurrent_draft_save_with_stale_timestamp_is_rejected() {
        let (_dir, store) = open_store();
        let v1 = store
            .save_showcase_draft("classic", &header_item("header.v1"))
            .unwrap();
        store
            .save_showcase_draft("classic", &header_item("header.v2"))
            .unwrap();
        let err = store
            .save_showcase_draft_versioned(
                "classic",
                &header_item("header.v3"),
                Some(&v1.updated_at),
            )
            .unwrap_err();
        assert!(showcase_is_conflict(&err), "{err:#}");
        assert_eq!(
            store
                .get_showcase()
                .unwrap()
                .composition
                .sections[0]
                .items[0]
                .element_id,
            "header.v2"
        );
    }

    #[test]
    fn apply_showcase_theme_rejects_after_concurrent_save() {
        let (_dir, store) = open_store();
        let stale = store.get_showcase().unwrap().updated_at;
        store
            .save_showcase_draft("classic", &header_item("header.race"))
            .unwrap();
        let err = store
            .apply_showcase_theme("ats-classic", &stale)
            .unwrap_err();
        assert!(showcase_is_conflict(&err));
    }

    #[test]
    fn apply_showcase_theme_changes_theme_not_composition() {
        let (_dir, store) = open_store();
        let composition = header_item("header.apply-default");
        store.save_showcase_draft("classic", &composition).unwrap();
        let before = store.capture_showcase().unwrap();
        let draft = store
            .apply_showcase_theme("ats-classic", &before.updated_at)
            .unwrap();
        assert_eq!(draft.theme_id, "ats-classic");
        assert_eq!(draft.composition, composition);
        assert_ne!(draft.updated_at, before.updated_at);
        assert!(store.list_showcase_publications().unwrap().is_empty());
    }
}
