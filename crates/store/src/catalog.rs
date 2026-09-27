use anyhow::{Context, Result};
use domain::ElementKind;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::Store;

#[derive(Debug, Clone, Serialize)]
pub struct ElementSummary {
    pub id: String,
    pub kind: String,
    pub created_at: String,
    pub latest_rev_number: i64,
    pub latest_revision_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevisionSummary {
    pub id: String,
    pub rev_number: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevisionDetail {
    pub id: String,
    pub element_id: String,
    pub rev_number: i64,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElementDetail {
    pub id: String,
    pub kind: String,
    pub created_at: String,
    pub revisions: Vec<RevisionSummary>,
    pub latest: Option<RevisionDetail>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRevisionRequest {
    pub payload: Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateElementRequest {
    pub id: String,
    pub kind: String,
    pub payload: Value,
}

impl Store {
    pub fn list_elements(&self, kind: Option<&str>) -> Result<Vec<ElementSummary>> {
        let conn = self.connection();
        let mut sql = String::from(
            "SELECT e.id, e.kind, e.created_at,
                    COALESCE(MAX(r.rev_number), 0) AS latest_rev,
                    (
                      SELECT r2.id FROM element_revisions r2
                      WHERE r2.element_id = e.id
                      ORDER BY r2.rev_number DESC LIMIT 1
                    ) AS latest_revision_id
             FROM elements e
             LEFT JOIN element_revisions r ON r.element_id = e.id
             WHERE 1=1",
        );
        if kind.is_some() {
            sql.push_str(" AND e.kind = ?1");
        }
        sql.push_str(" GROUP BY e.id ORDER BY e.kind, e.sort_order, e.id");

        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(k) = kind {
            stmt.query_map(params![k], map_element_summary)?
        } else {
            stmt.query_map([], map_element_summary)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_element(&self, element_id: &str) -> Result<Option<ElementDetail>> {
        let conn = self.connection();
        let Some((id, kind, created_at)) = conn
            .query_row(
                "SELECT id, kind, created_at FROM elements WHERE id = ?1",
                [element_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
        else {
            return Ok(None);
        };

        let mut rev_stmt = conn.prepare(
            "SELECT id, rev_number, created_at FROM element_revisions
             WHERE element_id = ?1 ORDER BY rev_number DESC",
        )?;
        let revisions: Vec<RevisionSummary> = rev_stmt
            .query_map([element_id], |row| {
                Ok(RevisionSummary {
                    id: row.get(0)?,
                    rev_number: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let latest = revisions
            .first()
            .map(|r| self.get_revision(&r.id))
            .transpose()?
            .flatten();

        Ok(Some(ElementDetail {
            id,
            kind,
            created_at,
            revisions,
            latest,
        }))
    }

    pub fn get_revision(&self, revision_id: &str) -> Result<Option<RevisionDetail>> {
        let conn = self.connection();
        conn.query_row(
            "SELECT id, element_id, rev_number, created_at, payload_json
             FROM element_revisions WHERE id = ?1",
            [revision_id],
            |row| {
                let payload_str: String = row.get(4)?;
                let payload: Value = serde_json::from_str(&payload_str).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                })?;
                Ok(RevisionDetail {
                    id: row.get(0)?,
                    element_id: row.get(1)?,
                    rev_number: row.get(2)?,
                    created_at: row.get(3)?,
                    payload,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn element_kind(&self, element_id: &str) -> Result<ElementKind> {
        let conn = self.connection();
        let kind: String = conn.query_row(
            "SELECT kind FROM elements WHERE id = ?1",
            [element_id],
            |row| row.get(0),
        )?;
        parse_kind(&kind).with_context(|| format!("unknown kind `{kind}` for element `{element_id}`"))
    }

    pub fn create_element(&self, req: &CreateElementRequest) -> Result<ElementDetail> {
        let kind = parse_kind(&req.kind)?;
        domain::validate_payload(kind, &req.payload)?;

        let id = req.id.trim();
        if id.is_empty() {
            anyhow::bail!("element id is required");
        }
        if id.contains(char::is_whitespace) {
            anyhow::bail!("element id must not contain whitespace");
        }

        let conn = self.connection();
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM elements WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        if exists > 0 {
            anyhow::bail!("element `{id}` already exists");
        }

        let now = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        let sort_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM elements WHERE kind = ?1",
            [req.kind.as_str()],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO elements (id, kind, created_at, sort_order) VALUES (?1, ?2, ?3, ?4)",
            params![id, req.kind, now, sort_order],
        )?;

        let rev_id = Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(&req.payload)?;
        conn.execute(
            "INSERT INTO element_revisions (id, element_id, rev_number, payload_json, created_at)
             VALUES (?1, ?2, 1, ?3, ?4)",
            params![rev_id, id, payload_json, now],
        )?;

        self.get_element(id)?
            .context("element missing after create")
    }

    pub fn create_revision(&self, element_id: &str, payload: Value) -> Result<RevisionDetail> {
        let kind = self.element_kind(element_id)?;
        domain::validate_payload(kind, &payload)?;

        let conn = self.connection();
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM elements WHERE id = ?1",
            [element_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            anyhow::bail!("unknown element `{element_id}`");
        }

        let now = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        let next_rev: i64 = conn.query_row(
            "SELECT COALESCE(MAX(rev_number), 0) + 1 FROM element_revisions WHERE element_id = ?1",
            [element_id],
            |row| row.get(0),
        )?;
        let rev_id = Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(&payload)?;

        conn.execute(
            "INSERT INTO element_revisions (id, element_id, rev_number, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rev_id, element_id, next_rev, payload_json, now],
        )?;

        self.get_revision(&rev_id)?
            .context("revision missing after insert")
    }
}

fn map_element_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<ElementSummary> {
    Ok(ElementSummary {
        id: row.get(0)?,
        kind: row.get(1)?,
        created_at: row.get(2)?,
        latest_rev_number: row.get(3)?,
        latest_revision_id: row.get(4)?,
    })
}

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

fn parse_kind(s: &str) -> Result<ElementKind> {
    kind_from_str(s).ok_or_else(|| anyhow::anyhow!("unknown element kind `{s}`"))
}

pub fn kind_from_str(s: &str) -> Option<ElementKind> {
    match s {
        "header" => Some(ElementKind::Header),
        "objective" => Some(ElementKind::Objective),
        "experience" => Some(ElementKind::Experience),
        "education" => Some(ElementKind::Education),
        "project" => Some(ElementKind::Project),
        "skills" => Some(ElementKind::Skills),
        _ => None,
    }
}
