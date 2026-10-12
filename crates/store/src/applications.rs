use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use domain::{
    validate_application_meta, validate_phase_and_workflow, Compensation, Composition,
    SubmissionManifest, DRAFT_WORKFLOW,
};
use similarity::{rank_similar, RankInput, RankedMatch};
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
    /// Super-status: `draft` (preparing) or `submitted` (sent at least once).
    pub phase: String,
    /// Pipeline label; `draft` when phase is draft, else a `SUBMITTED_WORKFLOWS` value.
    pub status: String,
    pub updated_at: String,
    pub canonical_submission_id: Option<String>,
    pub canonical_content_hash: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ListApplicationsQuery {
    /// Case-insensitive substring match on company or title.
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
    /// Application must include this tag.
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplicationDetail {
    pub id: String,
    pub company: String,
    pub title: String,
    pub posting_url: Option<String>,
    /// Pasted job description / posting body for keyword hints (read-only analysis).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub posting_text: String,
    pub source: Option<String>,
    pub phase: String,
    pub status: String,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employment_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default)]
    pub compensation: Compensation,
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
    pub posting_text: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Copy draft composition from an existing application instead of catalog default.
    #[serde(default)]
    pub clone_from_application_id: Option<String>,
    /// Copy composition from a saved preset.
    #[serde(default)]
    pub clone_from_preset_id: Option<String>,
    #[serde(default)]
    pub employment_type: Option<String>,
    #[serde(default)]
    pub work_mode: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub compensation: Option<Compensation>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QuickfillRequest {
    pub from_application_id: String,
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
    pub posting_text: Option<String>,
    #[serde(default)]
    pub source: Option<Option<String>>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub draft_composition: Option<Composition>,
    #[serde(default)]
    pub employment_type: Option<Option<String>>,
    #[serde(default)]
    pub work_mode: Option<Option<String>>,
    #[serde(default)]
    pub location: Option<Option<String>>,
    #[serde(default)]
    pub compensation: Option<Compensation>,
}

pub const DEFAULT_THEME_ID: &str = "classic";

impl Store {
    pub fn list_applications(&self, filter: &ListApplicationsQuery) -> Result<Vec<ApplicationSummary>> {
        let conn = self.connection();
        let q = filter
            .q
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        let status = filter
            .status
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        let tag = filter
            .tag
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        let phase = filter
            .phase
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_default();

        let mut stmt = conn.prepare(
            "SELECT a.id, a.company, a.title, a.posting_url, a.phase, a.status, a.updated_at, a.tags_json,
                    s.id, s.content_hash
             FROM applications a
             LEFT JOIN submissions s ON s.application_id = a.id AND s.is_canonical = 1
             WHERE (?1 = '' OR a.company LIKE '%' || ?1 || '%' OR a.title LIKE '%' || ?1 || '%')
               AND (?2 = '' OR a.status = ?2)
               AND (?3 = '' OR EXISTS (
                     SELECT 1 FROM json_each(a.tags_json) je WHERE je.value = ?3
                   ))
               AND (?4 = '' OR a.phase = ?4)
             ORDER BY a.updated_at DESC",
        )?;
        let rows = stmt.query_map(params![q, status, tag, phase], map_application_summary)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn rank_similar_applications(
        &self,
        application_id: &str,
        limit: usize,
    ) -> Result<Vec<RankedMatch>> {
        let source_detail = self
            .get_application(application_id)?
            .with_context(|| format!("unknown application `{application_id}`"))?;
        let inputs = self.load_rank_inputs()?;
        let source = RankInput {
            id: &source_detail.id,
            company: &source_detail.company,
            title: &source_detail.title,
            tags: &source_detail.tags,
            composition: &source_detail.draft_composition,
        };
        Ok(rank_similar(
            &source,
            &inputs
                .iter()
                .map(|i| RankInput {
                    id: &i.id,
                    company: &i.company,
                    title: &i.title,
                    tags: &i.tags,
                    composition: &i.composition,
                })
                .collect::<Vec<_>>(),
            limit,
        ))
    }

    pub fn quickfill_from_application(
        &self,
        target_id: &str,
        from_id: &str,
    ) -> Result<ApplicationDetail> {
        let source = self
            .get_application(from_id)?
            .with_context(|| format!("unknown application `{from_id}`"))?;
        self.update_application(
            target_id,
            UpdateApplicationRequest {
                draft_composition: Some(source.draft_composition),
                ..Default::default()
            },
        )
    }

    fn load_rank_inputs(&self) -> Result<Vec<RankInputRow>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT id, company, title, tags_json, draft_composition_json FROM applications",
        )?;
        let rows = stmt.query_map([], |row| {
            let tags_json: String = row.get(3)?;
            let draft_json: String = row.get(4)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                tags_json,
                draft_json,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, company, title, tags_json, draft_json) = row?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let composition: Composition = serde_json::from_str(&draft_json)
                .with_context(|| format!("parse draft for application `{id}`"))?;
            out.push(RankInputRow {
                id,
                company,
                title,
                tags,
                composition,
            });
        }
        Ok(out)
    }

    pub fn get_application(&self, application_id: &str) -> Result<Option<ApplicationDetail>> {
        let conn = self.connection();
        let row = conn
            .query_row(
                "SELECT id, company, title, posting_url, posting_text, source, phase, status, tags_json,
                        employment_type, work_mode, location, compensation_json,
                        draft_composition_json, created_at, updated_at
                 FROM applications WHERE id = ?1",
                [application_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                        row.get::<_, String>(14)?,
                        row.get::<_, String>(15)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            id,
            company,
            title,
            posting_url,
            posting_text,
            source,
            phase,
            status,
            tags_json,
            employment_type,
            work_mode,
            location,
            compensation_json,
            draft_json,
            created_at,
            updated_at,
        )) = row
        else {
            return Ok(None);
        };

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let compensation: Compensation =
            serde_json::from_str(&compensation_json).unwrap_or_default();
        let draft_composition: Composition = serde_json::from_str(&draft_json)
            .context("parse draft_composition_json")?;
        let submissions = self.list_submissions_for_app(&id)?;

        Ok(Some(ApplicationDetail {
            id,
            company,
            title,
            posting_url,
            posting_text,
            source,
            phase,
            status,
            tags,
            employment_type,
            work_mode,
            location,
            compensation,
            draft_composition,
            created_at,
            updated_at,
            submissions,
        }))
    }

    pub fn application_posting_hints(&self, application_id: &str) -> Result<Vec<domain::PostingHintRow>> {
        let app = self
            .get_application(application_id)?
            .with_context(|| format!("unknown application `{application_id}`"))?;
        let mut text = app.posting_text.clone();
        if text.trim().is_empty() {
            text = format!("{} {}", app.title, app.company);
            for tag in &app.tags {
                text.push(' ');
                text.push_str(tag);
            }
        }
        self.posting_skill_hints(&text, &app.draft_composition)
    }

    pub fn create_application(
        &self,
        req: CreateApplicationRequest,
        draft: Composition,
    ) -> Result<ApplicationDetail> {
        if let Some(ref st) = req.status {
            validate_phase_and_workflow("draft", st)?;
            if st != DRAFT_WORKFLOW {
                anyhow::bail!("new applications must start in draft phase (workflow `{st}`)");
            }
        }
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
        let phase = "draft".to_string();
        let status = DRAFT_WORKFLOW.to_string();
        validate_phase_and_workflow(&phase, &status)?;
        let tags_json = serde_json::to_string(&req.tags)?;
        let draft_json = serde_json::to_string(&draft)?;
        let employment_type = req.employment_type.clone();
        let work_mode = req.work_mode.clone();
        let location = req.location.clone();
        let compensation = req.compensation.clone().unwrap_or_default();
        validate_application_meta(
            employment_type.as_deref(),
            work_mode.as_deref(),
            location.as_deref(),
            &compensation,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        let compensation_json = serde_json::to_string(&compensation)?;

        let posting_text = req.posting_text.unwrap_or_default();

        conn.execute(
            "INSERT INTO applications
             (id, posting_url, posting_text, company, title, source, phase, status, tags_json,
              employment_type, work_mode, location, compensation_json,
              draft_composition_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                id,
                req.posting_url,
                posting_text,
                req.company,
                req.title,
                req.source,
                phase,
                status,
                tags_json,
                employment_type,
                work_mode,
                location,
                compensation_json,
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
        let posting_text = req.posting_text.unwrap_or(existing.posting_text);
        let source = match req.source {
            Some(v) => v,
            None => existing.source,
        };
        let phase_touched = req.phase.is_some();
        let status_touched = req.status.is_some();
        let mut phase = req.phase.unwrap_or(existing.phase);
        let mut status = req.status.unwrap_or(existing.status);

        if phase_touched || status_touched {
            (phase, status) = reconcile_phase_status(
                &phase,
                &status,
                !existing.submissions.is_empty(),
            )?;
        }
        validate_phase_and_workflow(&phase, &status)?;

        let tags = req.tags.unwrap_or(existing.tags);
        let draft = req.draft_composition.unwrap_or(existing.draft_composition);
        let employment_type = match req.employment_type {
            Some(v) => v,
            None => existing.employment_type,
        };
        let work_mode = match req.work_mode {
            Some(v) => v,
            None => existing.work_mode,
        };
        let location = match req.location {
            Some(v) => v,
            None => existing.location,
        };
        let compensation = req.compensation.unwrap_or(existing.compensation);
        validate_application_meta(
            employment_type.as_deref(),
            work_mode.as_deref(),
            location.as_deref(),
            &compensation,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

        let now = now_rfc3339();
        let tags_json = serde_json::to_string(&tags)?;
        let draft_json = serde_json::to_string(&draft)?;
        let compensation_json = serde_json::to_string(&compensation)?;

        let conn = self.connection();
        conn.execute(
            "UPDATE applications SET
                company = ?2, title = ?3, posting_url = ?4, posting_text = ?5, source = ?6, phase = ?7, status = ?8,
                tags_json = ?9, employment_type = ?10, work_mode = ?11, location = ?12,
                compensation_json = ?13, draft_composition_json = ?14, updated_at = ?15
             WHERE id = ?1",
            params![
                application_id,
                company,
                title,
                posting_url,
                posting_text,
                source,
                phase,
                status,
                tags_json,
                employment_type,
                work_mode,
                location,
                compensation_json,
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
            "UPDATE applications SET phase = 'submitted', status = 'submitted', updated_at = ?2 WHERE id = ?1",
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

struct RankInputRow {
    id: String,
    company: String,
    title: String,
    tags: Vec<String>,
    composition: Composition,
}

fn reconcile_phase_status(
    phase: &str,
    workflow: &str,
    has_submission: bool,
) -> Result<(String, String)> {
    let phase = phase.to_string();
    let workflow = workflow.to_string();

    if phase == "draft" {
        if has_submission {
            anyhow::bail!(
                "cannot return to draft phase after a submission; update pipeline workflow instead"
            );
        }
        return Ok((phase, DRAFT_WORKFLOW.to_string()));
    }

    if phase == "submitted" {
        if !has_submission {
            anyhow::bail!(
                "cannot set phase to submitted before the first Submit; use Submit to record what you sent"
            );
        }
        if workflow == DRAFT_WORKFLOW {
            return Ok((phase, "submitted".to_string()));
        }
        return Ok((phase, workflow));
    }

    anyhow::bail!("unknown phase `{phase}`")
}

fn map_application_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApplicationSummary> {
    let tags_json: String = row.get(7)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(ApplicationSummary {
        id: row.get(0)?,
        company: row.get(1)?,
        title: row.get(2)?,
        posting_url: row.get(3)?,
        phase: row.get(4)?,
        status: row.get(5)?,
        updated_at: row.get(6)?,
        tags,
        canonical_submission_id: row.get(8)?,
        canonical_content_hash: row.get(9)?,
    })
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}
