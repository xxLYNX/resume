use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use domain::{Composition, SubmissionManifest};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::Store;

#[derive(Debug, Clone, Serialize)]
pub struct ApplicationSummary {
    pub id: String,
    pub company: String,
    pub title: String,
    pub posting_url: Option<String>,
    pub status: String,
    pub updated_at: String,
    pub canonical_submission_id: Option<String>,
    pub canonical_content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplicationDetail {
    pub id: String,
    pub company: String,
    pub title: String,
    pub posting_url: Option<String>,
    pub source: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
    pub draft_composition: Composition,
    pub created_at: String,
    pub updated_at: String,
    pub submissions: Vec<SubmissionSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmissionSummary {
    pub id: String,
    pub submitted_at: String,
    pub content_hash: String,
    pub manifest_path: String,
    pub pdf_path: Option<String>,
    pub is_canonical: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmissionDetail {
    pub id: String,
    pub application_id: String,
    pub submitted_at: String,
    pub content_hash: String,
    pub manifest_path: String,
    pub pdf_path: Option<String>,
    pub is_canonical: bool,
    pub manifest: SubmissionManifest,
}

#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    pub company: String,
    pub title: String,
    #[serde(default)]
    pub posting_url: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateApplicationRequest {
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub posting_url: Option<Option<String>>,
    #[serde(default)]
    pub source: Option<Option<String>>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub draft_composition: Option<Composition>,
}

pub const DEFAULT_THEME_ID: &str = "classic";

impl Store {
    pub fn list_applications(&self) -> Result<Vec<ApplicationSummary>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT a.id, a.company, a.title, a.posting_url, a.status, a.updated_at,
                    s.id, s.content_hash
             FROM applications a
             LEFT JOIN submissions s ON s.application_id = a.id AND s.is_canonical = 1
             ORDER BY a.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ApplicationSummary {
                id: row.get(0)?,
                company: row.get(1)?,
                title: row.get(2)?,
                posting_url: row.get(3)?,
                status: row.get(4)?,
                updated_at: row.get(5)?,
                canonical_submission_id: row.get(6)?,
                canonical_content_hash: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_application(&self, application_id: &str) -> Result<Option<ApplicationDetail>> {
        let conn = self.connection();
        let row = conn
            .query_row(
                "SELECT id, company, title, posting_url, source, status, tags_json,
                        draft_composition_json, created_at, updated_at
                 FROM applications WHERE id = ?1",
                [application_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            id,
            company,
            title,
            posting_url,
            source,
            status,
            tags_json,
            draft_json,
            created_at,
            updated_at,
        )) = row
        else {
            return Ok(None);
        };

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let draft_composition: Composition = serde_json::from_str(&draft_json)
            .context("parse draft_composition_json")?;
        let submissions = self.list_submissions_for_app(&id)?;

        Ok(Some(ApplicationDetail {
            id,
            company,
            title,
            posting_url,
            source,
            status,
            tags,
            draft_composition,
            created_at,
            updated_at,
            submissions,
        }))
    }

    pub fn create_application(
        &self,
        req: CreateApplicationRequest,
        draft: Composition,
    ) -> Result<ApplicationDetail> {
        self.insert_application(req, draft)
    }

    fn insert_application(
        &self,
        req: CreateApplicationRequest,
        draft: Composition,
    ) -> Result<ApplicationDetail> {
        let conn = self.connection();
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let status = req.status.unwrap_or_else(|| "draft".to_string());
        let tags_json = serde_json::to_string(&req.tags)?;
        let draft_json = serde_json::to_string(&draft)?;

        conn.execute(
            "INSERT INTO applications
             (id, posting_url, company, title, source, status, tags_json,
              draft_composition_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                req.posting_url,
                req.company,
                req.title,
                req.source,
                status,
                tags_json,
                draft_json,
                now,
                now,
            ],
        )?;

        self.get_application(&id)?
            .context("application missing after insert")
    }

    pub fn update_application(
        &self,
        application_id: &str,
        req: UpdateApplicationRequest,
    ) -> Result<ApplicationDetail> {
        let existing = self
            .get_application(application_id)?
            .with_context(|| format!("unknown application `{application_id}`"))?;

        let company = req.company.unwrap_or(existing.company);
        let title = req.title.unwrap_or(existing.title);
        let posting_url = match req.posting_url {
            Some(v) => v,
            None => existing.posting_url,
        };
        let source = match req.source {
            Some(v) => v,
            None => existing.source,
        };
        let status = req.status.unwrap_or(existing.status);
        let tags = req.tags.unwrap_or(existing.tags);
        let draft = req.draft_composition.unwrap_or(existing.draft_composition);

        let now = now_rfc3339();
        let tags_json = serde_json::to_string(&tags)?;
        let draft_json = serde_json::to_string(&draft)?;

        let conn = self.connection();
        conn.execute(
            "UPDATE applications SET
                company = ?2, title = ?3, posting_url = ?4, source = ?5, status = ?6,
                tags_json = ?7, draft_composition_json = ?8, updated_at = ?9
             WHERE id = ?1",
            params![
                application_id,
                company,
                title,
                posting_url,
                source,
                status,
                tags_json,
                draft_json,
                now,
            ],
        )?;

        self.get_application(application_id)?
            .context("application missing after update")
    }

    pub fn delete_application(&self, application_id: &str) -> Result<bool> {
        let conn = self.connection();
        let n = conn.execute(
            "DELETE FROM applications WHERE id = ?1",
            [application_id],
        )?;
        Ok(n > 0)
    }

    pub fn submit_application(
        &self,
        application_id: &str,
        workspace_root: &Path,
        theme_id: &str,
        theme_files_hash: Option<String>,
        composition: &Composition,
        resolved: BTreeMap<String, Value>,
    ) -> Result<SubmissionDetail> {
        let manifests_dir = workspace_root.join("artifacts/manifests");
        let _app = self
            .get_application(application_id)?
            .with_context(|| format!("unknown application `{application_id}`"))?;

        let submitted_at = now_rfc3339();
        let manifest = SubmissionManifest::new(
            submitted_at.clone(),
            theme_id.to_string(),
            theme_files_hash,
            composition.clone(),
            resolved,
        );

        fs::create_dir_all(&manifests_dir)?;
        let submission_id = Uuid::new_v4().to_string();
        let manifest_filename = format!("{submission_id}.json");
        let manifest_path = manifests_dir.join(&manifest_filename);
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(&manifest_path, &manifest_json)?;

        let manifest_rel = format!(
            "artifacts/manifests/{manifest_filename}"
        );

        let conn = self.connection();
        conn.execute(
            "UPDATE submissions SET is_canonical = 0 WHERE application_id = ?1",
            [application_id],
        )?;
        conn.execute(
            "INSERT INTO submissions
             (id, application_id, manifest_path, content_hash, pdf_path, is_canonical, submitted_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 1, ?5)",
            params![
                submission_id,
                application_id,
                manifest_rel,
                manifest.content_hash,
                submitted_at,
            ],
        )?;

        conn.execute(
            "UPDATE applications SET status = 'submitted', updated_at = ?2 WHERE id = ?1",
            params![application_id, now_rfc3339()],
        )?;

        self.get_submission_with_root(workspace_root, &submission_id)?
            .context("submission missing after insert")
    }

    fn list_submissions_for_app(&self, application_id: &str) -> Result<Vec<SubmissionSummary>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT id, submitted_at, content_hash, manifest_path, pdf_path, is_canonical
             FROM submissions WHERE application_id = ?1 ORDER BY submitted_at DESC",
        )?;
        let rows = stmt.query_map([application_id], |row| {
            Ok(SubmissionSummary {
                id: row.get(0)?,
                submitted_at: row.get(1)?,
                content_hash: row.get(2)?,
                manifest_path: row.get(3)?,
                pdf_path: row.get(4)?,
                is_canonical: row.get::<_, i64>(5)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

/// Load manifest JSON; `manifest_path` is stored relative to workspace root.
pub fn load_manifest_at(workspace_root: &Path, manifest_rel: &str) -> Result<SubmissionManifest> {
    let path = workspace_root.join(manifest_rel);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read manifest {}", path.display()))?;
    serde_json::from_str(&text).context("parse submission manifest")
}

pub fn read_submission_manifest(
    workspace_root: &Path,
    submission: &SubmissionSummary,
) -> Result<SubmissionManifest> {
    load_manifest_at(workspace_root, &submission.manifest_path)
}

impl Store {
    pub fn get_submission_with_root(
        &self,
        workspace_root: &Path,
        submission_id: &str,
    ) -> Result<Option<SubmissionDetail>> {
        let conn = self.connection();
        let row = conn
            .query_row(
                "SELECT id, application_id, submitted_at, content_hash, manifest_path,
                        pdf_path, is_canonical
                 FROM submissions WHERE id = ?1",
                [submission_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            id,
            application_id,
            submitted_at,
            content_hash,
            manifest_path,
            pdf_path,
            is_canonical,
        )) = row
        else {
            return Ok(None);
        };

        let manifest = load_manifest_at(workspace_root, &manifest_path)?;
        Ok(Some(SubmissionDetail {
            id,
            application_id,
            submitted_at,
            content_hash,
            manifest_path,
            pdf_path,
            is_canonical: is_canonical != 0,
            manifest,
        }))
    }

    pub fn get_canonical_submission_with_root(
        &self,
        workspace_root: &Path,
        application_id: &str,
    ) -> Result<Option<SubmissionDetail>> {
        let conn = self.connection();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM submissions
                 WHERE application_id = ?1 AND is_canonical = 1
                 ORDER BY submitted_at DESC LIMIT 1",
                [application_id],
                |row| row.get(0),
            )
            .optional()?;
        match id {
            Some(sub_id) => self.get_submission_with_root(workspace_root, &sub_id),
            None => Ok(None),
        }
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}
