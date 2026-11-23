//! Independent job postings (opportunities), immutable snapshots, and interpretations.

use anyhow::{Context, Result};
use domain::{
    extraction_outcome_usable, outcome_list_label, parse_extraction_outcome,
    validate_application_meta, validate_compensation, FetchedPostingPayload,
    RETRIEVAL_KIND_GREENHOUSE,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use super::Store;

/// Normalize URL for deduplication: https, strip fragment, trim trailing slash path.
pub fn normalize_posting_url(input: &str) -> Result<String> {
    let mut u = Url::parse(input.trim()).map_err(|e| anyhow::anyhow!("invalid URL: {e}"))?;
    u.set_fragment(None);
    if u.scheme() == "http" {
        let _ = u.set_scheme("https");
    }
    let s = u.as_str().trim_end_matches('/').to_string();
    Ok(s)
}

#[derive(Debug, Clone, Serialize)]
pub struct PostingSummary {
    pub id: String,
    pub normalized_url: String,
    pub user_state: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_label: Option<String>,
    /// When the current interpretation is unreviewed, timestamp of the latest reviewed row (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostingSnapshotSummary {
    pub id: String,
    pub posting_id: String,
    pub fetched_at: String,
    pub request_url: String,
    pub final_url: String,
    pub http_status: i64,
    pub content_type: String,
    pub body_sha256: String,
    pub extractor_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostingInterpretationDetail {
    pub id: String,
    pub posting_id: String,
    pub snapshot_id: String,
    pub created_at: String,
    pub reviewed_at: Option<String>,
    /// Machine extraction (immutable for this interpretation row).
    pub machine_suggested: FetchedPostingPayload,
    /// User edits; partial object merged over machine for display.
    pub user_overrides: Value,
    /// Merged view for UI (`user_overrides` wins).
    pub merged: FetchedPostingPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extraction_diagnostics: Vec<String>,
}

/// When the current interpretation is unreviewed, the most recent reviewed row (if any).
#[derive(Debug, Clone, Serialize)]
pub struct PriorReviewedInterpretation {
    pub interpretation_id: String,
    pub snapshot_id: String,
    pub reviewed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostingFetchAttemptSummary {
    pub id: String,
    pub attempted_at: String,
    pub retrieval_kind: String,
    pub http_status: i64,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostingDetail {
    pub posting: PostingSummary,
    pub snapshots: Vec<PostingSnapshotSummary>,
    /// Latest interpretation row (may be failed extraction).
    pub current: Option<PostingInterpretationDetail>,
    /// Latest interpretation whose outcome is `recognized` or `partial`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_usable: Option<PostingInterpretationDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_reviewed: Option<PriorReviewedInterpretation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_fetch_attempt: Option<PostingFetchAttemptSummary>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostingInterpretationRequest {
    /// Interpretation the client is editing (required for conflict detection when stale).
    #[serde(default)]
    pub interpretation_id: Option<String>,
    #[serde(default)]
    pub user_overrides: Option<Value>,
    /// When set, marks interpretation reviewed (optional).
    #[serde(default)]
    pub mark_reviewed: Option<bool>,
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

fn parse_fetched_payload(v: &Value) -> Result<FetchedPostingPayload> {
    serde_json::from_value(v.clone()).context("parse FetchedPostingPayload")
}

fn merge_payload(machine: &FetchedPostingPayload, user: &Value) -> Result<FetchedPostingPayload> {
    let mut base = serde_json::to_value(machine)?;
    if let Some(u) = user.as_object() {
        if let Some(base_obj) = base.as_object_mut() {
            for (k, val) in u {
                if val.is_null() {
                    base_obj.remove(k);
                } else {
                    base_obj.insert(k.clone(), val.clone());
                }
            }
        }
    }
    serde_json::from_value(base).context("merge payload")
}

fn display_from_merged(m: &FetchedPostingPayload) -> (Option<String>, Option<String>) {
    (m.title.clone(), m.employer.clone())
}

fn ui_extraction_diagnostics(diagnostics: &[String]) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|d| {
            !d.starts_with("http_status:")
                && !d.starts_with("content_type:")
                && !d.starts_with(domain::DIAG_OUTCOME_PREFIX)
        })
        .cloned()
        .collect()
}

impl Store {
    pub fn count_applications(&self) -> Result<i64> {
        let n: i64 =
            self.connection()
                .query_row("SELECT COUNT(*) FROM applications", [], |row| row.get(0))?;
        Ok(n)
    }

    pub fn get_posting_by_normalized_url(&self, normalized_url: &str) -> Result<Option<String>> {
        let conn = self.connection();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM postings WHERE normalized_url = ?1",
                [normalized_url],
                |row| row.get(0),
            )
            .optional()?;
        Ok(id)
    }

    /// Append snapshot + new interpretation. If posting exists for URL, append only.
    /// `raw_body_html` must be complete (caller enforces max size). `machine_suggested` from extract.
    pub fn save_posting_fetch(
        &self,
        request_url: &str,
        final_url: &str,
        redirect_chain_json: &Value,
        http_status: i64,
        content_type: &str,
        raw_body_html: &str,
        body_sha256: &str,
        extractor_id: &str,
        extractor_version: &str,
        diagnostics: &[String],
        machine_suggested: &FetchedPostingPayload,
    ) -> Result<PostingDetail> {
        self.save_posting_fetch_extended(
            request_url,
            final_url,
            redirect_chain_json,
            http_status,
            content_type,
            raw_body_html,
            body_sha256,
            extractor_id,
            extractor_version,
            diagnostics,
            machine_suggested,
            "http_html",
            "{}",
            None,
        )
    }

    pub fn save_posting_fetch_extended(
        &self,
        request_url: &str,
        final_url: &str,
        redirect_chain_json: &Value,
        http_status: i64,
        content_type: &str,
        raw_body_html: &str,
        body_sha256: &str,
        extractor_id: &str,
        extractor_version: &str,
        diagnostics: &[String],
        machine_suggested: &FetchedPostingPayload,
        retrieval_kind: &str,
        provider_provenance_json: &str,
        identity_url: Option<&str>,
    ) -> Result<PostingDetail> {
        let normalized = normalize_posting_url(identity_url.unwrap_or(final_url))?;
        let conn = self.connection();
        let now = now_rfc3339();

        conn.execute_batch("BEGIN IMMEDIATE;")?;
        let tx = (|| -> Result<String> {
            let posting_id = match self.get_posting_by_normalized_url(&normalized)? {
                Some(id) => id,
                None => {
                    let id = Uuid::new_v4().to_string();
                    conn.execute(
                    "INSERT INTO postings (id, normalized_url, user_state, created_at, updated_at)
                     VALUES (?1, ?2, 'active', ?3, ?4)",
                    params![id, normalized, now, now],
                )?;
                    id
                }
            };

            let snap_id = Uuid::new_v4().to_string();
            let diagnostics_json = serde_json::to_string(&diagnostics.to_vec())?;
            conn.execute(
                "INSERT INTO posting_snapshots (
                id, posting_id, fetched_at, request_url, final_url, redirect_chain_json,
                http_status, content_type, body_sha256, raw_body_html,
                extractor_id, extractor_version, diagnostics_json,
                retrieval_kind, provider_provenance_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    snap_id,
                    posting_id,
                    now,
                    request_url,
                    final_url,
                    serde_json::to_string(redirect_chain_json)?,
                    http_status,
                    content_type,
                    body_sha256,
                    raw_body_html,
                    extractor_id,
                    extractor_version,
                    diagnostics_json,
                    retrieval_kind,
                    provider_provenance_json
                ],
            )?;

            let interp_id = Uuid::new_v4().to_string();
            let machine_json = serde_json::to_string(machine_suggested)?;
            let user_json = serde_json::to_string(&json!({}))?;
            conn.execute(
                "INSERT INTO posting_interpretations (
                id, posting_id, snapshot_id, created_at, reviewed_at, machine_json, user_json
             ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
                params![interp_id, posting_id, snap_id, now, machine_json, user_json],
            )?;

            conn.execute(
                "UPDATE postings SET updated_at = ?1 WHERE id = ?2",
                params![now, posting_id],
            )?;
            Ok(posting_id)
        })();
        match tx {
            Ok(pid) => {
                conn.execute_batch("COMMIT;")?;
                self.get_posting(&pid)?
                    .context("posting missing after insert")
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    }

    /// Acceptance tests only: remove all opportunity postings (not applications).
    pub fn clear_all_postings_for_acceptance(&self) -> Result<()> {
        let conn = self.connection();
        conn.execute_batch(
            "DELETE FROM posting_fetch_attempts;
             DELETE FROM posting_interpretations;
             DELETE FROM posting_snapshots;
             DELETE FROM postings;",
        )?;
        Ok(())
    }

    /// Provider adapter ingest: always records fetch attempt; snapshot only for archivable JSON body.
    pub fn ingest_provider_fetch(
        &self,
        ingest_url: &str,
        api_request_url: &str,
        final_url: &str,
        redirect_chain_json: &Value,
        http_status: i64,
        content_type: &str,
        snapshot_stored: Option<&str>,
        snapshot_raw_bytes: Option<&[u8]>,
        body_sha256: Option<&str>,
        provenance_json: &str,
        extractor_id: &str,
        extractor_version: &str,
        diagnostics: &[String],
        machine_suggested: &FetchedPostingPayload,
        outcome: &str,
        attempt_response_body: Option<&str>,
        retrieval_kind: &str,
    ) -> Result<PostingDetail> {
        let normalized = normalize_posting_url(ingest_url)?;
        let conn = self.connection();
        let now = now_rfc3339();
        conn.execute_batch("BEGIN IMMEDIATE;")?;
        let tx = (|| -> Result<String> {
            let posting_id = match self.get_posting_by_normalized_url(&normalized)? {
                Some(id) => id,
                None => {
                    let id = Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO postings (id, normalized_url, user_state, created_at, updated_at)
                         VALUES (?1, ?2, 'active', ?3, ?4)",
                        params![id, normalized, now, now],
                    )?;
                    id
                }
            };

            let attempt_id = Uuid::new_v4().to_string();
            let attempt_diags = serde_json::to_string(diagnostics)?;
            let mut snapshot_id: Option<String> = None;

            let create_snapshot = matches!(snapshot_stored, Some(raw) if !raw.is_empty())
                && http_status >= 200
                && http_status < 300;

            if let (true, Some(stored)) = (create_snapshot, snapshot_stored) {
                let snap_id = Uuid::new_v4().to_string();
                let hash = body_sha256.context("body sha required with snapshot")?;
                if let Some(raw) = snapshot_raw_bytes {
                    let expect = domain::sha256_hex(raw);
                    if expect != hash {
                        anyhow::bail!("body_sha256 does not match snapshot bytes");
                    }
                }
                let diagnostics_json = serde_json::to_string(diagnostics)?;
                conn.execute(
                    "INSERT INTO posting_snapshots (
                    id, posting_id, fetched_at, request_url, final_url, redirect_chain_json,
                    http_status, content_type, body_sha256, raw_body_html,
                    extractor_id, extractor_version, diagnostics_json,
                    retrieval_kind, provider_provenance_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        snap_id,
                        posting_id,
                        now,
                        api_request_url,
                        final_url,
                        serde_json::to_string(redirect_chain_json)?,
                        http_status,
                        content_type,
                        hash,
                        stored,
                        extractor_id,
                        extractor_version,
                        diagnostics_json,
                        retrieval_kind,
                        provenance_json
                    ],
                )?;
                let interp_id = Uuid::new_v4().to_string();
                let machine_json = serde_json::to_string(machine_suggested)?;
                conn.execute(
                    "INSERT INTO posting_interpretations (
                    id, posting_id, snapshot_id, created_at, reviewed_at, machine_json, user_json
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
                    params![interp_id, posting_id, snap_id, now, machine_json, "{}"],
                )?;
                snapshot_id = Some(snap_id);
            }

            let attempt_body = if snapshot_id.is_some() {
                None
            } else {
                attempt_response_body
            };

            conn.execute(
                "INSERT INTO posting_fetch_attempts (
                    id, posting_id, attempted_at, retrieval_kind, request_url, final_url,
                    redirect_chain_json, http_status, response_content_type, response_body,
                    outcome, diagnostics_json, snapshot_id
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                params![
                    attempt_id,
                    posting_id,
                    now,
                    retrieval_kind,
                    api_request_url,
                    final_url,
                    serde_json::to_string(redirect_chain_json)?,
                    http_status,
                    content_type,
                    attempt_body,
                    outcome,
                    attempt_diags,
                    snapshot_id
                ],
            )?;

            conn.execute(
                "UPDATE postings SET updated_at = ?1 WHERE id = ?2",
                params![now, posting_id],
            )?;
            Ok(posting_id)
        })();
        match tx {
            Ok(pid) => {
                conn.execute_batch("COMMIT;")?;
                self.get_posting(&pid)?
                    .context("posting missing after greenhouse ingest")
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    }

    pub fn ingest_greenhouse_fetch(
        &self,
        ingest_url: &str,
        api_request_url: &str,
        final_url: &str,
        redirect_chain_json: &Value,
        http_status: i64,
        content_type: &str,
        snapshot_stored: Option<&str>,
        snapshot_raw_bytes: Option<&[u8]>,
        body_sha256: Option<&str>,
        provenance_json: &str,
        extractor_id: &str,
        extractor_version: &str,
        diagnostics: &[String],
        machine_suggested: &FetchedPostingPayload,
        outcome: &str,
        attempt_response_body: Option<&str>,
    ) -> Result<PostingDetail> {
        self.ingest_provider_fetch(
            ingest_url,
            api_request_url,
            final_url,
            redirect_chain_json,
            http_status,
            content_type,
            snapshot_stored,
            snapshot_raw_bytes,
            body_sha256,
            provenance_json,
            extractor_id,
            extractor_version,
            diagnostics,
            machine_suggested,
            outcome,
            attempt_response_body,
            RETRIEVAL_KIND_GREENHOUSE,
        )
    }

    pub fn ingest_greenhouse_transport_failure(
        &self,
        ingest_url: &str,
        api_request_url: &str,
        diagnostics: &[String],
    ) -> Result<PostingDetail> {
        self.ingest_provider_transport_failure(
            ingest_url,
            api_request_url,
            diagnostics,
            RETRIEVAL_KIND_GREENHOUSE,
        )
    }

    /// Record provider transport failure (attempt only).
    pub fn ingest_provider_transport_failure(
        &self,
        ingest_url: &str,
        api_request_url: &str,
        diagnostics: &[String],
        retrieval_kind: &str,
    ) -> Result<PostingDetail> {
        let normalized = normalize_posting_url(ingest_url)?;
        let conn = self.connection();
        let now = now_rfc3339();
        conn.execute_batch("BEGIN IMMEDIATE;")?;
        let tx = (|| -> Result<String> {
            let posting_id = match self.get_posting_by_normalized_url(&normalized)? {
                Some(id) => id,
                None => {
                    let id = Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO postings (id, normalized_url, user_state, created_at, updated_at)
                         VALUES (?1, ?2, 'active', ?3, ?4)",
                        params![id, normalized, now, now],
                    )?;
                    id
                }
            };
            let attempt_id = Uuid::new_v4().to_string();
            let attempt_diags = serde_json::to_string(diagnostics)?;
            conn.execute(
                "INSERT INTO posting_fetch_attempts (
                    id, posting_id, attempted_at, retrieval_kind, request_url, final_url,
                    redirect_chain_json, http_status, response_content_type, response_body,
                    outcome, diagnostics_json, snapshot_id
                 ) VALUES (?1,?2,?3,?4,?5,?6,'[]',0,NULL,NULL,'extraction_failed',?7,NULL)",
                params![
                    attempt_id,
                    posting_id,
                    now,
                    retrieval_kind,
                    api_request_url,
                    api_request_url,
                    attempt_diags
                ],
            )?;
            conn.execute(
                "UPDATE postings SET updated_at = ?1 WHERE id = ?2",
                params![now, posting_id],
            )?;
            Ok(posting_id)
        })();
        match tx {
            Ok(pid) => {
                conn.execute_batch("COMMIT;")?;
                Ok(self.get_posting(&pid)?.ok_or_else(|| {
                    anyhow::anyhow!("posting missing after transport failure ingest")
                })?)
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    }

    pub fn list_postings(&self) -> Result<Vec<PostingSummary>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT p.id, p.normalized_url, p.user_state, p.updated_at
             FROM postings p
             ORDER BY p.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (id, normalized_url, user_state, updated_at) = r?;
            let (display_title, display_company) = self
                .current_interpretation_merged(&id)?
                .map(|m| display_from_merged(&m))
                .unwrap_or((None, None));
            let (extraction_outcome, list_label) =
                self.list_metadata_for_posting(&id, display_title.is_some())?;
            let prior_reviewed_at = self
                .load_prior_reviewed(&id, self.load_current_interpretation(&id)?.as_ref())?
                .map(|p| p.reviewed_at);
            out.push(PostingSummary {
                id,
                normalized_url,
                user_state,
                updated_at,
                display_title,
                display_company,
                extraction_outcome,
                list_label,
                prior_reviewed_at,
            });
        }
        Ok(out)
    }

    fn current_interpretation_merged(
        &self,
        posting_id: &str,
    ) -> Result<Option<FetchedPostingPayload>> {
        let current = self.load_current_interpretation(posting_id)?;
        let use_row = if let Some(ref c) = current {
            if extraction_outcome_usable(c.extraction_outcome.as_deref()) {
                current
            } else {
                self.load_last_usable_interpretation(posting_id)?
            }
        } else {
            self.load_last_usable_interpretation(posting_id)?
        };
        Ok(use_row.map(|d| d.merged))
    }

    fn assert_interpretation_belongs_to_posting(
        &self,
        interpretation_id: &str,
        posting_id: &str,
    ) -> Result<()> {
        let conn = self.connection();
        let ok: Option<String> = conn
            .query_row(
                "SELECT posting_id FROM posting_interpretations WHERE id = ?1",
                [interpretation_id],
                |row| row.get(0),
            )
            .optional()?;
        match ok {
            Some(pid) if pid == posting_id => Ok(()),
            Some(_) => anyhow::bail!("interpretation does not belong to posting"),
            None => anyhow::bail!("interpretation not found"),
        }
    }

    /// Interpretation PATCH/Mark-reviewed target: avoid writing user edits onto a failed latest row.
    fn interpretation_id_for_user_edit(&self, posting_id: &str) -> Result<String> {
        let current = self.load_current_interpretation(posting_id)?;
        if let Some(ref c) = current {
            if extraction_outcome_usable(c.extraction_outcome.as_deref()) {
                return Ok(c.id.clone());
            }
        }
        if let Some(u) = self.load_last_usable_interpretation(posting_id)? {
            return Ok(u.id);
        }
        if let Some(c) = current {
            return Ok(c.id);
        }
        anyhow::bail!("no interpretation for posting")
    }

    fn load_current_interpretation(
        &self,
        posting_id: &str,
    ) -> Result<Option<PostingInterpretationDetail>> {
        let conn = self.connection();
        let row: Option<(
            String,
            String,
            String,
            String,
            Option<String>,
            String,
            String,
        )> = conn
            .query_row(
                "SELECT i.id, i.posting_id, i.snapshot_id, i.created_at, i.reviewed_at,
                        i.machine_json, i.user_json
                 FROM posting_interpretations i
                 WHERE i.posting_id = ?1
                 ORDER BY i.created_at DESC, i.rowid DESC
                 LIMIT 1",
                [posting_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()?;
        let Some((iid, pid, sid, created, reviewed, machine_s, user_s)) = row else {
            return Ok(None);
        };
        let machine_val: Value = serde_json::from_str(&machine_s)?;
        let user_val: Value = serde_json::from_str(&user_s)?;
        let machine_suggested: FetchedPostingPayload = parse_fetched_payload(&machine_val)?;
        let merged = merge_payload(&machine_suggested, &user_val)?;
        let snapshot_diags = self.load_snapshot_diagnostics(&sid)?;
        let extraction_outcome = parse_extraction_outcome(&snapshot_diags);
        let extraction_diagnostics = ui_extraction_diagnostics(&snapshot_diags);
        Ok(Some(PostingInterpretationDetail {
            id: iid,
            posting_id: pid,
            snapshot_id: sid,
            created_at: created,
            reviewed_at: reviewed,
            machine_suggested,
            user_overrides: user_val,
            merged,
            extraction_outcome,
            extraction_diagnostics,
        }))
    }

    fn load_snapshot_diagnostics(&self, snapshot_id: &str) -> Result<Vec<String>> {
        let conn = self.connection();
        let raw: String = conn.query_row(
            "SELECT diagnostics_json FROM posting_snapshots WHERE id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )?;
        serde_json::from_str(&raw).with_context(|| format!("parse diagnostics for {snapshot_id}"))
    }

    fn list_metadata_for_posting(
        &self,
        posting_id: &str,
        has_display_title: bool,
    ) -> Result<(Option<String>, Option<String>)> {
        let Some(cur) = self.load_current_interpretation(posting_id)? else {
            return Ok((None, None));
        };
        let diags = self.load_snapshot_diagnostics(&cur.snapshot_id)?;
        let outcome = parse_extraction_outcome(&diags);
        let list_label = if has_display_title {
            None
        } else {
            outcome
                .as_deref()
                .map(outcome_list_label)
                .map(str::to_string)
        };
        Ok((outcome, list_label))
    }

    pub fn get_posting(&self, posting_id: &str) -> Result<Option<PostingDetail>> {
        let conn = self.connection();
        let posting_row: Option<(String, String, String, String, String)> = conn
            .query_row(
                "SELECT id, normalized_url, user_state, created_at, updated_at
                 FROM postings WHERE id = ?1",
                [posting_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((id, normalized_url, user_state, _created_at, updated_at)) = posting_row else {
            return Ok(None);
        };

        let mut snap_stmt = conn.prepare(
            "SELECT id, posting_id, fetched_at, request_url, final_url, http_status,
                    content_type, body_sha256, extractor_version
             FROM posting_snapshots WHERE posting_id = ?1
             ORDER BY fetched_at ASC",
        )?;
        let snapshots: Vec<PostingSnapshotSummary> = snap_stmt
            .query_map([posting_id], |row| {
                Ok(PostingSnapshotSummary {
                    id: row.get(0)?,
                    posting_id: row.get(1)?,
                    fetched_at: row.get(2)?,
                    request_url: row.get(3)?,
                    final_url: row.get(4)?,
                    http_status: row.get(5)?,
                    content_type: row.get(6)?,
                    body_sha256: row.get(7)?,
                    extractor_version: row.get(8)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        let current = self.load_current_interpretation(posting_id)?;
        let last_usable = self.load_last_usable_interpretation(posting_id)?;
        let latest_fetch_attempt = self.load_latest_fetch_attempt(posting_id)?;
        let prior_reviewed = self.load_prior_reviewed(posting_id, current.as_ref())?;
        let display_source = if current
            .as_ref()
            .and_then(|c| c.extraction_outcome.as_deref())
            .is_some_and(|o| extraction_outcome_usable(Some(o)))
        {
            current.as_ref()
        } else {
            last_usable.as_ref().or(current.as_ref())
        };
        let (display_title, display_company) = display_source
            .map(|c| display_from_merged(&c.merged))
            .unwrap_or((None, None));
        let (extraction_outcome, list_label) =
            self.list_metadata_for_posting(posting_id, display_title.is_some())?;

        Ok(Some(PostingDetail {
            posting: PostingSummary {
                id,
                normalized_url,
                user_state,
                updated_at,
                display_title,
                display_company,
                extraction_outcome,
                list_label,
                prior_reviewed_at: prior_reviewed.as_ref().map(|p| p.reviewed_at.clone()),
            },
            snapshots,
            current,
            last_usable,
            prior_reviewed,
            latest_fetch_attempt,
        }))
    }

    fn load_last_usable_interpretation(
        &self,
        posting_id: &str,
    ) -> Result<Option<PostingInterpretationDetail>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT i.id, i.posting_id, i.snapshot_id, i.created_at, i.reviewed_at,
                    i.machine_json, i.user_json
             FROM posting_interpretations i
             WHERE i.posting_id = ?1
             ORDER BY i.created_at DESC, i.rowid DESC",
        )?;
        let rows = stmt.query_map([posting_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        for r in rows {
            let (iid, pid, sid, created, reviewed, machine_s, user_s) = r?;
            let machine_val: Value = serde_json::from_str(&machine_s)?;
            let user_val: Value = serde_json::from_str(&user_s)?;
            let machine_suggested: FetchedPostingPayload = parse_fetched_payload(&machine_val)?;
            let merged = merge_payload(&machine_suggested, &user_val)?;
            let snapshot_diags = self.load_snapshot_diagnostics(&sid)?;
            let extraction_outcome = parse_extraction_outcome(&snapshot_diags);
            if extraction_outcome_usable(extraction_outcome.as_deref()) {
                return Ok(Some(PostingInterpretationDetail {
                    id: iid,
                    posting_id: pid,
                    snapshot_id: sid,
                    created_at: created,
                    reviewed_at: reviewed,
                    machine_suggested,
                    user_overrides: user_val,
                    merged,
                    extraction_outcome,
                    extraction_diagnostics: ui_extraction_diagnostics(&snapshot_diags),
                }));
            }
        }
        Ok(None)
    }

    fn load_latest_fetch_attempt(
        &self,
        posting_id: &str,
    ) -> Result<Option<PostingFetchAttemptSummary>> {
        let conn = self.connection();
        let row: Option<(String, String, String, i64, String, String, Option<String>)> = conn
            .query_row(
                "SELECT id, attempted_at, retrieval_kind, http_status, outcome,
                        diagnostics_json, snapshot_id
                 FROM posting_fetch_attempts
                 WHERE posting_id = ?1
                 ORDER BY attempted_at DESC, rowid DESC
                 LIMIT 1",
                [posting_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(
            |(id, attempted_at, retrieval_kind, http_status, outcome, diags_raw, snapshot_id)| {
                let diagnostics: Vec<String> = serde_json::from_str(&diags_raw).unwrap_or_default();
                PostingFetchAttemptSummary {
                    id,
                    attempted_at,
                    retrieval_kind,
                    http_status,
                    outcome,
                    diagnostics: ui_extraction_diagnostics(&diagnostics),
                    snapshot_id,
                }
            },
        ))
    }

    fn load_prior_reviewed(
        &self,
        posting_id: &str,
        current: Option<&PostingInterpretationDetail>,
    ) -> Result<Option<PriorReviewedInterpretation>> {
        if current.map(|c| c.reviewed_at.is_some()).unwrap_or(false) {
            return Ok(None);
        }
        let conn = self.connection();
        let current_id = current.map(|c| c.id.as_str());
        let row: Option<(String, String, String)> = if let Some(cid) = current_id {
            conn.query_row(
                "SELECT id, snapshot_id, reviewed_at FROM posting_interpretations
                 WHERE posting_id = ?1 AND reviewed_at IS NOT NULL AND id != ?2
                 ORDER BY reviewed_at DESC, rowid DESC
                 LIMIT 1",
                params![posting_id, cid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
        } else {
            conn.query_row(
                "SELECT id, snapshot_id, reviewed_at FROM posting_interpretations
                 WHERE posting_id = ?1 AND reviewed_at IS NOT NULL
                 ORDER BY reviewed_at DESC, rowid DESC
                 LIMIT 1",
                [posting_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
        };
        Ok(row.map(
            |(interpretation_id, snapshot_id, reviewed_at)| PriorReviewedInterpretation {
                interpretation_id,
                snapshot_id,
                reviewed_at,
            },
        ))
    }

    pub fn get_snapshot_retrieval_kind(&self, snapshot_id: &str) -> Result<Option<String>> {
        let conn = self.connection();
        conn.query_row(
            "SELECT retrieval_kind FROM posting_snapshots WHERE id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn get_snapshot_html(&self, snapshot_id: &str) -> Result<Option<String>> {
        let conn = self.connection();
        let raw: Option<String> = conn
            .query_row(
                "SELECT raw_body_html FROM posting_snapshots WHERE id = ?1",
                [snapshot_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(raw)
    }

    pub fn update_posting_interpretation(
        &self,
        posting_id: &str,
        req: UpdatePostingInterpretationRequest,
    ) -> Result<PostingDetail> {
        let conn = self.connection();
        let target_id = self.interpretation_id_for_user_edit(posting_id)?;
        let interp_id = if let Some(ref client_id) = req.interpretation_id {
            if client_id != &target_id {
                anyhow::bail!("stale_interpretation: client={client_id} target={target_id}");
            }
            self.assert_interpretation_belongs_to_posting(client_id, posting_id)?;
            client_id.clone()
        } else {
            self.assert_interpretation_belongs_to_posting(&target_id, posting_id)?;
            target_id
        };

        let existing_user: String = conn.query_row(
            "SELECT user_json FROM posting_interpretations WHERE id = ?1",
            [&interp_id],
            |row| row.get(0),
        )?;

        let overrides_changed = req.user_overrides.is_some();
        let mut user_val: Value = serde_json::from_str(&existing_user)?;
        if let Some(patch) = req.user_overrides {
            if let Some(u) = user_val.as_object_mut() {
                if let Some(uo) = patch.as_object() {
                    for (k, v) in uo {
                        if v.is_null() {
                            u.remove(k);
                        } else {
                            u.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
        }

        let machine_s: String = conn.query_row(
            "SELECT machine_json FROM posting_interpretations WHERE id = ?1",
            [&interp_id],
            |row| row.get(0),
        )?;
        let machine_val: Value = serde_json::from_str(&machine_s)?;
        let machine_suggested: FetchedPostingPayload = parse_fetched_payload(&machine_val)?;
        let merged = merge_payload(&machine_suggested, &user_val)?;

        if let Some(emp) = merged.employment_type.as_deref() {
            if !domain::EMPLOYMENT_TYPES.iter().any(|&x| x == emp) {
                anyhow::bail!("invalid employment_type `{emp}`");
            }
        }
        if let Some(wm) = merged.work_mode.as_deref() {
            if !domain::WORK_MODES.iter().any(|&x| x == wm) {
                anyhow::bail!("invalid work_mode `{wm}`");
            }
        }
        let compensation = merged.compensation.clone().unwrap_or_default();
        validate_compensation(&compensation).map_err(|e| anyhow::anyhow!("{e}"))?;
        validate_application_meta(
            merged.employment_type.as_deref(),
            merged.work_mode.as_deref(),
            merged.location.as_deref(),
            &compensation,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

        let existing_reviewed: Option<String> = conn.query_row(
            "SELECT reviewed_at FROM posting_interpretations WHERE id = ?1",
            [&interp_id],
            |row| row.get(0),
        )?;

        let now = now_rfc3339();
        let reviewed = if req.mark_reviewed == Some(true) {
            Some(now.clone())
        } else if overrides_changed && existing_reviewed.is_some() {
            None
        } else {
            existing_reviewed
        };

        conn.execute(
            "UPDATE posting_interpretations SET user_json = ?1, reviewed_at = ?2 WHERE id = ?3",
            params![serde_json::to_string(&user_val)?, reviewed, interp_id],
        )?;

        conn.execute(
            "UPDATE postings SET updated_at = ?1 WHERE id = ?2",
            params![now, posting_id],
        )?;

        self.get_posting(posting_id)?.context("posting missing")
    }
}

#[cfg(test)]
mod tests {
    use super::UpdatePostingInterpretationRequest;
    use crate::{CreateApplicationRequest, Store};
    use domain::Composition;
    use tempfile::TempDir;

    fn empty_composition() -> Composition {
        Composition { sections: vec![] }
    }

    #[test]
    fn posting_ingest_does_not_create_application() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let apps0 = store.count_applications().unwrap();
        assert_eq!(apps0, 0);

        store
            .create_application(
                CreateApplicationRequest {
                    company: "Co".into(),
                    title: "T".into(),
                    posting_url: None,
                    posting_text: None,
                    source: None,
                    status: None,
                    tags: vec![],
                    clone_from_application_id: None,
                    clone_from_preset_id: None,
                    employment_type: None,
                    work_mode: None,
                    location: None,
                    compensation: None,
                },
                empty_composition(),
            )
            .unwrap();
        assert_eq!(store.count_applications().unwrap(), 1);

        let html = r#"<!DOCTYPE html><html><head>
<script type="application/ld+json">
{"@context":"https://schema.org/","@type":"JobPosting","title":"T1","hiringOrganization":{"@type":"Organization","name":"C1"}}
</script></head><body><p>desc</p></body></html>"#;
        let machine = domain::extract_from_html(html).payload;
        store
            .save_posting_fetch(
                "https://example.com/jobs/1",
                "https://example.com/jobs/1",
                &serde_json::json!(["https://example.com/jobs/1"]),
                200,
                "text/html",
                html,
                &domain::sha256_hex(html.as_bytes()),
                "test",
                domain::POSTING_EXTRACTOR_VERSION,
                &["ok".into()],
                &machine,
            )
            .unwrap();

        assert_eq!(
            store.count_applications().unwrap(),
            1,
            "saving posting must not insert applications"
        );
    }

    #[test]
    fn second_fetch_appends_snapshot_keeps_history() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let html1 = "<html><body>one</body></html>";
        let html2 = "<html><body>two</body></html>";
        let m1 = domain::extract_from_html(html1).payload;
        let m2 = domain::extract_from_html(html2).payload;
        let d = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html1,
                &domain::sha256_hex(html1.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m1,
            )
            .unwrap();
        let pid = d.posting.id.clone();
        let d2 = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html2,
                &domain::sha256_hex(html2.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m2,
            )
            .unwrap();
        assert_eq!(d2.posting.id, pid);
        assert_eq!(d2.snapshots.len(), 2);
        assert_ne!(d2.snapshots[0].body_sha256, d2.snapshots[1].body_sha256);
    }

    #[test]
    fn foreign_keys_reject_orphan_snapshot() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let conn = store.connection();
        let err = conn.execute(
            "INSERT INTO posting_snapshots (
                id, posting_id, fetched_at, request_url, final_url, redirect_chain_json,
                http_status, content_type, body_sha256, raw_body_html,
                extractor_id, extractor_version, diagnostics_json
             ) VALUES ('s1', 'missing-posting', 't', 'u', 'u', '[]', 200, 'text/html', 'h', '<html></html>', 'x', '1', '[]')",
            [],
        );
        assert!(err.is_err(), "orphan snapshot insert must fail with FK on");
    }

    #[test]
    fn save_posting_fetch_rolls_back_when_interpretation_blocked() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        store
            .connection()
            .execute_batch(
                "CREATE TRIGGER block_test_interp BEFORE INSERT ON posting_interpretations
             FOR EACH ROW WHEN NEW.extractor_id = 'BLOCK_TEST'
             BEGIN SELECT RAISE(ABORT, 'block interpretation'); END;",
            )
            .unwrap();
        let html = "<html><body>x</body></html>";
        let machine = domain::extract_from_html(html).payload;
        let err = store.save_posting_fetch(
            "https://example.com/j",
            "https://example.com/j",
            &serde_json::json!([]),
            200,
            "text/html",
            html,
            &domain::sha256_hex(html.as_bytes()),
            "BLOCK_TEST",
            domain::POSTING_EXTRACTOR_VERSION,
            &[],
            &machine,
        );
        assert!(err.is_err());
        let n: i64 = store
            .connection()
            .query_row("SELECT COUNT(*) FROM postings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "failed ingest must not commit posting row");
        let s: i64 = store
            .connection()
            .query_row("SELECT COUNT(*) FROM posting_snapshots", [], |r| r.get(0))
            .unwrap();
        assert_eq!(s, 0);
    }

    #[test]
    fn refetch_starts_with_empty_user_overrides() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let html1 = "<html><body>one</body></html>";
        let m1 = domain::extract_from_html(html1).payload;
        let d = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html1,
                &domain::sha256_hex(html1.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m1,
            )
            .unwrap();
        let pid = d.posting.id;
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: None,
                    user_overrides: Some(serde_json::json!({"title": "Edited"})),
                    mark_reviewed: None,
                },
            )
            .unwrap();
        let html2 = "<html><body>two</body></html>";
        let m2 = domain::extract_from_html(html2).payload;
        store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html2,
                &domain::sha256_hex(html2.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m2,
            )
            .unwrap();
        let detail = store.get_posting(&pid).unwrap().unwrap();
        let cur = detail.current.unwrap();
        assert!(cur.user_overrides.as_object().unwrap().is_empty());
    }

    #[test]
    fn editing_reviewed_without_remark_clears_review() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let html = "<html><body>x</body></html>";
        let machine = domain::extract_from_html(html).payload;
        let d = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html,
                &domain::sha256_hex(html.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &machine,
            )
            .unwrap();
        let pid = d.posting.id;
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: None,
                    user_overrides: None,
                    mark_reviewed: Some(true),
                },
            )
            .unwrap();
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: None,
                    user_overrides: Some(serde_json::json!({"title": "Tweaked"})),
                    mark_reviewed: None,
                },
            )
            .unwrap();
        let cur = store.get_posting(&pid).unwrap().unwrap().current.unwrap();
        assert!(cur.reviewed_at.is_none());
    }

    #[test]
    fn prior_reviewed_surfaces_after_mark_reviewed_and_refetch() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let html1 = r#"<!DOCTYPE html><html><head><script type="application/ld+json">
{"@type":"JobPosting","title":"V1","hiringOrganization":{"name":"Co"}}
</script></head><body></body></html>"#;
        let m1 = domain::extract_from_html(html1).payload;
        let d = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html1,
                &domain::sha256_hex(html1.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m1,
            )
            .unwrap();
        let pid = d.posting.id.clone();
        let interp1 = d.current.as_ref().unwrap().id.clone();
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: None,
                    user_overrides: Some(serde_json::json!({"title": "Edited title"})),
                    mark_reviewed: None,
                },
            )
            .unwrap();
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: None,
                    user_overrides: None,
                    mark_reviewed: Some(true),
                },
            )
            .unwrap();
        let html2 = r#"<!DOCTYPE html><html><head><script type="application/ld+json">
{"@type":"JobPosting","title":"V2","hiringOrganization":{"name":"Co"}}
</script></head><body></body></html>"#;
        let m2 = domain::extract_from_html(html2).payload;
        let after_refetch = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html2,
                &domain::sha256_hex(html2.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m2,
            )
            .unwrap();
        let prior = after_refetch
            .prior_reviewed
            .as_ref()
            .expect("prior_reviewed after refetch");
        assert_eq!(prior.interpretation_id, interp1);
        assert!(after_refetch
            .current
            .as_ref()
            .unwrap()
            .reviewed_at
            .is_none());
        let json = serde_json::to_value(&after_refetch).unwrap();
        assert!(json.get("prior_reviewed").is_some());
        let list = store.list_postings().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].prior_reviewed_at.is_some());
    }

    #[test]
    fn patch_targets_last_usable_after_greenhouse_mismatch_refetch() {
        use std::fs;
        use std::path::PathBuf;

        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let ingest = "https://job-boards.greenhouse.io/flexport/jobs/8009198";
        let api =
            "https://boards-api.greenhouse.io/v1/boards/flexport/jobs/8009198?pay_transparency=true";
        let fx = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../domain/tests/fixtures/greenhouse/flexport-8009198.json");
        let good = fs::read_to_string(&fx).unwrap();
        let extracted = domain::interpret_greenhouse_job_json(&good, "8009198");
        let hash = domain::sha256_hex(good.as_bytes());
        let outcome = domain::parse_extraction_outcome(&extracted.diagnostics).unwrap();
        let d1 = store
            .ingest_greenhouse_fetch(
                ingest,
                api,
                api,
                &serde_json::json!([]),
                200,
                "application/json",
                Some(good.as_str()),
                Some(good.as_bytes()),
                Some(&hash),
                "{}",
                domain::GREENHOUSE_ADAPTER_ID,
                domain::GREENHOUSE_ADAPTER_VERSION,
                &extracted.diagnostics,
                &extracted.payload,
                &outcome,
                None,
            )
            .unwrap();
        let pid = d1.posting.id.clone();
        let usable_id = d1.current.as_ref().unwrap().id.clone();
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: None,
                    user_overrides: None,
                    mark_reviewed: Some(true),
                },
            )
            .unwrap();
        let mut bad: serde_json::Value = serde_json::from_str(&good).unwrap();
        bad["id"] = serde_json::json!(9999999);
        let bad_s = serde_json::to_string(&bad).unwrap();
        let bad_ext = domain::interpret_greenhouse_job_json(&bad_s, "8009198");
        let bad_outcome = domain::parse_extraction_outcome(&bad_ext.diagnostics).unwrap();
        let _d2 = store
            .ingest_greenhouse_fetch(
                ingest,
                api,
                api,
                &serde_json::json!([]),
                200,
                "application/json",
                Some(bad_s.as_str()),
                Some(bad_s.as_bytes()),
                Some(&domain::sha256_hex(bad_s.as_bytes())),
                "{}",
                domain::GREENHOUSE_ADAPTER_ID,
                domain::GREENHOUSE_ADAPTER_VERSION,
                &bad_ext.diagnostics,
                &bad_ext.payload,
                &bad_outcome,
                None,
            )
            .unwrap();
        store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: Some(usable_id.clone()),
                    user_overrides: Some(serde_json::json!({"title": "Owner edit"})),
                    mark_reviewed: None,
                },
            )
            .unwrap();
        let detail = store.get_posting(&pid).unwrap().unwrap();
        assert_ne!(
            detail.current.as_ref().unwrap().id,
            usable_id,
            "current is failed extraction"
        );
        assert_eq!(detail.last_usable.as_ref().unwrap().id, usable_id);
        let failed_user: String = store
            .connection()
            .query_row(
                "SELECT user_json FROM posting_interpretations WHERE id = ?1",
                [detail.current.as_ref().unwrap().id.clone()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(failed_user, "{}");
        let usable_user: String = store
            .connection()
            .query_row(
                "SELECT user_json FROM posting_interpretations WHERE id = ?1",
                [usable_id.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        let uv: serde_json::Value = serde_json::from_str(&usable_user).unwrap();
        assert_eq!(uv["title"], "Owner edit");
        // Editing cleared reviewed_at on the usable row (L semantics); failed current stays unreviewed.
        assert!(detail.last_usable.as_ref().unwrap().reviewed_at.is_none());
    }

    #[test]
    fn patch_stale_interpretation_id_rejected() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let html = r#"<!DOCTYPE html><html><head><script type="application/ld+json">
{"@type":"JobPosting","title":"V1","hiringOrganization":{"name":"Co"}}
</script></head><body></body></html>"#;
        let m = domain::extract_from_html(html).payload;
        let d = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html,
                &domain::sha256_hex(html.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m,
            )
            .unwrap();
        let pid = d.posting.id;
        let first_id = d.current.as_ref().unwrap().id.clone();
        let html2 = r#"<!DOCTYPE html><html><head><script type="application/ld+json">
{"@type":"JobPosting","title":"V2","hiringOrganization":{"name":"Co"}}
</script></head><body></body></html>"#;
        let m2 = domain::extract_from_html(html2).payload;
        let d2 = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html2,
                &domain::sha256_hex(html2.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m2,
            )
            .unwrap();
        let stale = first_id;
        let current = d2.current.as_ref().unwrap().id.clone();
        assert_ne!(stale, current);
        let err = store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: Some(stale),
                    user_overrides: Some(serde_json::json!({"title": "stale"})),
                    mark_reviewed: None,
                },
            )
            .unwrap_err();
        assert!(err.to_string().contains("stale_interpretation"));
    }

    #[test]
    fn mark_reviewed_stale_interpretation_id_rejected() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("jobdesk.db");
        let store = Store::open(&db).unwrap();
        let html = r#"<!DOCTYPE html><html><head><script type="application/ld+json">
{"@type":"JobPosting","title":"V1","hiringOrganization":{"name":"Co"}}
</script></head><body></body></html>"#;
        let m = domain::extract_from_html(html).payload;
        let d = store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html,
                &domain::sha256_hex(html.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m,
            )
            .unwrap();
        let pid = d.posting.id;
        let stale = d.current.as_ref().unwrap().id.clone();
        let html2 = r#"<!DOCTYPE html><html><head><script type="application/ld+json">
{"@type":"JobPosting","title":"V2","hiringOrganization":{"name":"Co"}}
</script></head><body></body></html>"#;
        let m2 = domain::extract_from_html(html2).payload;
        store
            .save_posting_fetch(
                "https://example.com/job",
                "https://example.com/job",
                &serde_json::json!([]),
                200,
                "text/html",
                html2,
                &domain::sha256_hex(html2.as_bytes()),
                "t",
                domain::POSTING_EXTRACTOR_VERSION,
                &[],
                &m2,
            )
            .unwrap();
        let err = store
            .update_posting_interpretation(
                &pid,
                UpdatePostingInterpretationRequest {
                    interpretation_id: Some(stale),
                    user_overrides: None,
                    mark_reviewed: Some(true),
                },
            )
            .unwrap_err();
        assert!(err.to_string().contains("stale_interpretation"));
    }

    #[test]
    fn snapshot_sha256_matches_raw_bytes() {
        let bytes = br#"{"id":1,"title":"X","company_name":"Y","content":"<p>Role</p>"}"#;
        let stored = domain::snapshot_body_for_storage(bytes).0;
        let hash = domain::sha256_hex(bytes);
        let dir = TempDir::new().unwrap();
        let store = Store::open(&dir.path().join("jobdesk.db")).unwrap();
        let ext = domain::interpret_greenhouse_job_json(std::str::from_utf8(bytes).unwrap(), "1");
        let outcome = domain::parse_extraction_outcome(&ext.diagnostics).unwrap();
        let d = store
            .ingest_greenhouse_fetch(
                "https://job-boards.greenhouse.io/t/jobs/1",
                "https://boards-api.greenhouse.io/v1/boards/t/jobs/1?pay_transparency=true",
                "https://boards-api.greenhouse.io/v1/boards/t/jobs/1?pay_transparency=true",
                &serde_json::json!([]),
                200,
                "application/json",
                Some(stored.as_str()),
                Some(bytes.as_slice()),
                Some(&hash),
                "{}",
                domain::GREENHOUSE_ADAPTER_ID,
                "1",
                &ext.diagnostics,
                &ext.payload,
                &outcome,
                None,
            )
            .unwrap();
        assert_eq!(d.snapshots[0].body_sha256, hash);
    }

    #[test]
    fn invalid_utf8_snapshot_stores_base64_and_matching_hash() {
        let bytes: &[u8] = &[0xff, b'{', b'}'];
        let stored = domain::snapshot_body_for_storage(bytes).0;
        let hash = domain::sha256_hex(bytes);
        assert!(stored.starts_with(domain::SNAPSHOT_BODY_PREFIX_BASE64));
        let dir = TempDir::new().unwrap();
        let store = Store::open(&dir.path().join("jobdesk.db")).unwrap();
        let outcome = domain::OUTCOME_EXTRACTION_FAILED;
        let diags = vec![format!("{}{}", domain::DIAG_OUTCOME_PREFIX, outcome)];
        let d = store
            .ingest_greenhouse_fetch(
                "https://job-boards.greenhouse.io/t/jobs/1",
                "https://boards-api.greenhouse.io/v1/boards/t/jobs/1?pay_transparency=true",
                "https://boards-api.greenhouse.io/v1/boards/t/jobs/1?pay_transparency=true",
                &serde_json::json!([]),
                200,
                "application/json",
                Some(stored.as_str()),
                Some(bytes),
                Some(&hash),
                "{}",
                domain::GREENHOUSE_ADAPTER_ID,
                "1",
                &diags,
                &domain::FetchedPostingPayload::default(),
                outcome,
                None,
            )
            .unwrap();
        let snap = &d.snapshots[0];
        assert_eq!(snap.body_sha256, hash);
        let raw: String = store.get_snapshot_html(&snap.id).unwrap().unwrap();
        assert_eq!(raw, stored);
    }

    #[test]
    fn ingest_provider_fetch_sets_workday_retrieval_kind() {
        let raw = include_str!("../../domain/tests/fixtures/workday/crowdstrike-r30561.json");
        let bytes = raw.as_bytes();
        let stored = domain::snapshot_body_for_storage(bytes).0;
        let hash = domain::sha256_hex(bytes);
        let ext = domain::interpret_workday_cxs_json(raw, Some("R30561"));
        let outcome = domain::parse_extraction_outcome(&ext.diagnostics).unwrap();
        let dir = TempDir::new().unwrap();
        let store = Store::open(&dir.path().join("jobdesk.db")).unwrap();
        let ingest =
            "https://crowdstrike.wd5.myworkdayjobs.com/en-US/crowdstrikecareers/job/USA---Remote/_R30561";
        let cxs = "https://crowdstrike.wd5.myworkdayjobs.com/wday/cxs/crowdstrike/crowdstrikecareers/job/USA---Remote/_R30561";
        let d = store
            .ingest_provider_fetch(
                ingest,
                cxs,
                cxs,
                &serde_json::json!([]),
                200,
                "application/json",
                Some(stored.as_str()),
                Some(bytes),
                Some(&hash),
                "{}",
                domain::WORKDAY_ADAPTER_ID,
                "1",
                &ext.diagnostics,
                &ext.payload,
                &outcome,
                None,
                domain::RETRIEVAL_KIND_WORKDAY,
            )
            .unwrap();
        let snap_id = d.snapshots[0].id.clone();
        assert_eq!(
            store
                .get_snapshot_retrieval_kind(&snap_id)
                .unwrap()
                .as_deref(),
            Some(domain::RETRIEVAL_KIND_WORKDAY)
        );
    }

    #[test]
    fn provider_fetch_404_no_snapshot_attempt_has_body() {
        let err_body = r#"{"errorCode":"S21","httpStatus":404,"message":"not found"}"#;
        let dir = TempDir::new().unwrap();
        let store = Store::open(&dir.path().join("jobdesk.db")).unwrap();
        let diags = vec![
            format!(
                "{}{}",
                domain::DIAG_OUTCOME_PREFIX,
                domain::OUTCOME_SOURCE_UNAVAILABLE
            ),
            "http_status:404".into(),
        ];
        let d = store
            .ingest_provider_fetch(
                "https://crowdstrike.wd5.myworkdayjobs.com/en-US/crowdstrikecareers/job/nope",
                "https://crowdstrike.wd5.myworkdayjobs.com/wday/cxs/crowdstrike/crowdstrikecareers/job/nope",
                "https://crowdstrike.wd5.myworkdayjobs.com/wday/cxs/crowdstrike/crowdstrikecareers/job/nope",
                &serde_json::json!([]),
                404,
                "application/json",
                None,
                None,
                None,
                "{}",
                domain::WORKDAY_ADAPTER_ID,
                "1",
                &diags,
                &domain::FetchedPostingPayload::default(),
                domain::OUTCOME_SOURCE_UNAVAILABLE,
                Some(err_body),
                domain::RETRIEVAL_KIND_WORKDAY,
            )
            .unwrap();
        assert!(d.snapshots.is_empty());
        let attempt = d.latest_fetch_attempt.as_ref().unwrap();
        assert_eq!(attempt.http_status, 404);
        assert_eq!(attempt.outcome, domain::OUTCOME_SOURCE_UNAVAILABLE);
        let body: Option<String> = store
            .connection()
            .query_row(
                "SELECT response_body FROM posting_fetch_attempts WHERE id = ?1",
                [&attempt.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(body.as_deref().is_some_and(|b| b.contains("not found")));
    }
}
