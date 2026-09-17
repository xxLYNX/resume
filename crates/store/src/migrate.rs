use anyhow::Result;
use rusqlite::Connection;

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

    conn.execute(
        "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('schema_version', '4')",
        [],
    )?;
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
