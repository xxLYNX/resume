use anyhow::{Context, Result};
use domain::{normalize_payload_dates, Composition, ElementKind};

use crate::catalog::kind_from_str;
use rusqlite::{params, Connection};
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
    if version < 9 {
        migrate_skill_catalog(conn)?;
    }
    if version < 10 {
        migrate_competency_graph_v10(conn)?;
    }
    if version < 11 {
        migrate_retire_skills_main(conn)?;
    }
    if version < 12 {
        migrate_catalog_seed_v12(conn)?;
    }
    if version < 13 {
        migrate_posting_text_and_framework_refs_v13(conn)?;
    }
    if version < 14 {
        migrate_catalog_v14_no_auto_seed(conn)?;
    }
    if version < 15 {
        migrate_strip_project_technologies_v15(conn)?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('schema_version', '15')",
        [],
    )?;
    Ok(())
}

fn migrate_strip_project_technologies_v15(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT er.id, er.payload_json
         FROM element_revisions er
         JOIN elements e ON e.id = er.element_id
         WHERE e.kind = 'project'",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (rev_id, payload_json) in rows {
        let mut payload: Value =
            serde_json::from_str(&payload_json).unwrap_or(Value::Object(Default::default()));
        if let Some(obj) = payload.as_object_mut() {
            if obj.remove("technologies").is_some() {
                conn.execute(
                    "UPDATE element_revisions SET payload_json = ?1 WHERE id = ?2",
                    params![serde_json::to_string(&payload)?, rev_id],
                )?;
            }
        }
    }
    Ok(())
}

fn migrate_catalog_seed_v12(conn: &Connection) -> Result<()> {
    crate::skills::upsert_catalog_seed(conn)?;
    crate::skills::seed_tool_competency_edges(conn)?;
    Ok(())
}

fn migrate_posting_text_and_framework_refs_v13(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "applications", "posting_text", "TEXT NOT NULL DEFAULT ''")?;
    crate::skills::apply_framework_refs_seed(conn)?;
    Ok(())
}

fn migrate_catalog_v14_no_auto_seed(_conn: &Connection) -> Result<()> {
    Ok(())
}

/// Drop legacy `skills.main` elements and remove composable `skills` sections from stored JSON.
fn migrate_retire_skills_main(conn: &Connection) -> Result<()> {
    strip_skills_sections_from_compositions(conn, "applications", "draft_composition_json")?;
    strip_skills_sections_from_compositions(conn, "presets", "composition_json")?;

    let skill_ids: Vec<String> = conn
        .prepare("SELECT id FROM elements WHERE kind = 'skills'")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for id in skill_ids {
        conn.execute(
            "DELETE FROM element_revisions WHERE element_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM elements WHERE id = ?1", params![id])?;
    }
    Ok(())
}

fn strip_skills_sections_from_compositions(
    conn: &Connection,
    table: &str,
    column: &str,
) -> Result<()> {
    let sql = format!("SELECT id, {column} FROM {table}");
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (id, json) in rows {
        let mut composition: Composition =
            serde_json::from_str(&json).with_context(|| format!("parse {table}.{column} `{id}`"))?;
        let before = composition.sections.len();
        composition
            .sections
            .retain(|s| s.section_key != "skills");
        if composition.sections.len() == before {
            continue;
        }
        let updated = serde_json::to_string(&composition)?;
        let update = format!("UPDATE {table} SET {column} = ?1 WHERE id = ?2");
        conn.execute(&update, params![updated, id])?;
    }
    Ok(())
}

fn migrate_skill_catalog(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS skill_nodes (
            id TEXT PRIMARY KEY NOT NULL,
            kind TEXT NOT NULL,
            label TEXT NOT NULL,
            parent_id TEXT REFERENCES skill_nodes(id),
            aliases_json TEXT NOT NULL DEFAULT '[]',
            pdf_groups_json TEXT NOT NULL DEFAULT '[\"other\"]',
            framework_refs_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE TABLE IF NOT EXISTS revision_skill_links (
            revision_id TEXT NOT NULL REFERENCES element_revisions(id) ON DELETE CASCADE,
            skill_id TEXT NOT NULL REFERENCES skill_nodes(id),
            axis TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'asserted',
            PRIMARY KEY (revision_id, skill_id, axis)
        );

        CREATE INDEX IF NOT EXISTS idx_revision_skill_links_revision
            ON revision_skill_links(revision_id);
        CREATE INDEX IF NOT EXISTS idx_skill_nodes_kind
            ON skill_nodes(kind);
        ",
    )?;
    crate::skills::seed_skill_nodes_if_empty(conn)?;
    Ok(())
}

fn migrate_competency_graph_v10(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE skill_nodes SET kind = 'tool' WHERE kind IN ('with', 'tool')",
        [],
    )?;
    conn.execute(
        "UPDATE skill_nodes SET kind = 'competency' WHERE kind IN ('what', 'competency')",
        [],
    )?;
    for (old, new) in [
        ("activity.reverse-engineering", "competency.reverse-engineering"),
        ("activity.security-audit", "competency.security-audit"),
        ("activity.code-review", "competency.code-review"),
    ] {
        conn.execute(
            "UPDATE revision_skill_links SET skill_id = ?1 WHERE skill_id = ?2",
            params![new, old],
        )?;
        conn.execute(
            "UPDATE skill_nodes SET id = ?1 WHERE id = ?2",
            params![new, old],
        )?;
    }
    conn.execute(
        "DELETE FROM revision_skill_links WHERE skill_id LIKE 'purpose.%'
         OR skill_id IN (
           SELECT id FROM skill_nodes WHERE kind IN ('for', 'purpose')
         )",
        [],
    )?;
    conn.execute(
        "DELETE FROM skill_nodes WHERE kind IN ('for', 'purpose')",
        [],
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS element_skill_links (
            element_id TEXT NOT NULL REFERENCES elements(id) ON DELETE CASCADE,
            skill_id TEXT NOT NULL REFERENCES skill_nodes(id),
            source TEXT NOT NULL DEFAULT 'asserted',
            PRIMARY KEY (element_id, skill_id)
        );
        CREATE INDEX IF NOT EXISTS idx_element_skill_links_skill
            ON element_skill_links(skill_id);

        CREATE TABLE IF NOT EXISTS tool_competency_edges (
            tool_id TEXT NOT NULL REFERENCES skill_nodes(id),
            competency_id TEXT NOT NULL REFERENCES skill_nodes(id),
            source TEXT NOT NULL DEFAULT 'import',
            PRIMARY KEY (tool_id, competency_id)
        );
        ",
    )?;

    let has_element_links: i64 = conn.query_row(
        "SELECT COUNT(*) FROM element_skill_links",
        [],
        |r| r.get(0),
    )?;
    if has_element_links == 0 {
        conn.execute(
            "INSERT OR IGNORE INTO element_skill_links (element_id, skill_id, source)
             SELECT er.element_id, l.skill_id, l.source
             FROM revision_skill_links l
             JOIN element_revisions er ON er.id = l.revision_id
             JOIN skill_nodes n ON n.id = l.skill_id
             WHERE n.kind IN ('tool', 'competency')
               AND l.source = 'asserted'",
            [],
        )?;
    }

    conn.execute_batch(
        "
        DROP TABLE IF EXISTS revision_skill_links;
        ",
    )?;

    add_column_if_missing(conn, "elements", "parent_element_id", "TEXT REFERENCES elements(id)")?;
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
