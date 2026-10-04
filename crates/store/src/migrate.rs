use anyhow::{Context, Result};
use domain::{normalize_payload_dates, ElementKind};

use crate::catalog::kind_from_str;
use rusqlite::Connection;
use serde_json::Value;

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS elements (
            id TEXT PRIMARY KEY NOT NULL,
            kind TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS element_revisions (
            id TEXT PRIMARY KEY NOT NULL,
            element_id TEXT NOT NULL REFERENCES elements(id),
            rev_number INTEGER NOT NULL,
            payload_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(element_id, rev_number)
        );

        CREATE TABLE IF NOT EXISTS schema_meta (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );
        ",
    )?;

    add_column_if_missing(conn, "elements", "sort_order", "INTEGER NOT NULL DEFAULT 0")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS applications (
            id TEXT PRIMARY KEY NOT NULL,
            posting_url TEXT,
            company TEXT NOT NULL,
            title TEXT NOT NULL,
            source TEXT,
            status TEXT NOT NULL,
            tags_json TEXT NOT NULL DEFAULT '[]',
            draft_composition_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS submissions (
            id TEXT PRIMARY KEY NOT NULL,
            application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
            manifest_path TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            pdf_path TEXT,
            is_canonical INTEGER NOT NULL DEFAULT 0,
            submitted_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_submissions_application
            ON submissions(application_id);
        ",
    )?;

    add_column_if_missing(conn, "applications", "phase", "TEXT NOT NULL DEFAULT 'draft'")?;

    conn.execute(
        "UPDATE applications SET phase = 'submitted' WHERE status != 'draft'",
        [],
    )?;
    conn.execute(
        "UPDATE applications SET phase = 'draft', status = 'draft' WHERE status = 'draft'",
        [],
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS presets (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            theme_id TEXT NOT NULL DEFAULT 'classic',
            composition_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        ",
    )?;

    add_column_if_missing(conn, "applications", "employment_type", "TEXT")?;
    add_column_if_missing(conn, "applications", "work_mode", "TEXT")?;
    add_column_if_missing(conn, "applications", "location", "TEXT")?;
    add_column_if_missing(
        conn,
        "applications",
        "compensation_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;

    let version = schema_version(conn)?;
    if version < 7 {
        migrate_payload_date_ranges(conn)?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('schema_version', '7')",
        [],
    )?;
    Ok(())
}

fn schema_version(conn: &Connection) -> Result<i64> {
    let mut stmt = conn.prepare("SELECT value FROM schema_meta WHERE key = 'schema_version'")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let s: String = row.get(0)?;
        return Ok(s.parse().unwrap_or(0));
    }
    Ok(0)
}

fn migrate_payload_date_ranges(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT er.id, e.kind, er.payload_json
         FROM element_revisions er
         JOIN elements e ON e.id = er.element_id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (rev_id, kind_str, payload_json) in rows {
        let Some(kind) = kind_from_str(&kind_str) else {
            continue;
        };
        if !matches!(
            kind,
            ElementKind::Education | ElementKind::Experience | ElementKind::Project
        ) {
            continue;
        }
        let mut payload: Value =
            serde_json::from_str(&payload_json).context("parse revision payload")?;
        let before = payload.clone();
        normalize_payload_dates(&mut payload)
            .with_context(|| format!("normalize dates for revision `{rev_id}`"))?;
        if payload == before {
            continue;
        }
        domain::validate_payload(kind, &payload)?;
        let updated = serde_json::to_string(&payload)?;
        conn.execute(
            "UPDATE element_revisions SET payload_json = ?1 WHERE id = ?2",
            rusqlite::params![updated, rev_id],
        )?;
    }
    Ok(())
}

fn add_column_if_missing(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if names.iter().any(|n| n == column) {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}
