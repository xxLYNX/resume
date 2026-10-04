use std::path::Path;

use anyhow::{Context, Result};
use domain::ElementKind;
use rusqlite::{params, Connection};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Store;

#[derive(Debug, Clone, Copy, Default)]
pub struct ImportOptions {
    /// When set, YAML content is appended as a new revision for existing elements.
    pub force: bool,
}

#[derive(Debug, Default)]
pub struct ImportStats {
    pub created: usize,
    pub skipped: usize,
    pub appended: usize,
}

pub fn import_legacy_content(
    project_root: &Path,
    store: &Store,
    options: ImportOptions,
) -> Result<ImportStats> {
    let conn = store.connection();
    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();

    let mut stats = ImportStats::default();

    import_singleton(
        conn,
        &now,
        &project_root.join("content/header.yaml"),
        ElementKind::Header,
        "header.main",
        0,
        options,
        &mut stats,
    )?;

    import_singleton(
        conn,
        &now,
        &project_root.join("content/objective.yaml"),
        ElementKind::Objective,
        "objective.main",
        0,
        options,
        &mut stats,
    )?;

    import_singleton(
        conn,
        &now,
        &project_root.join("content/skills.yaml"),
        ElementKind::Skills,
        "skills.main",
        0,
        options,
        &mut stats,
    )?;

    import_list(
        conn,
        &now,
        &project_root.join("content/education.yaml"),
        "education",
        ElementKind::Education,
        options,
        &mut stats,
        |item| {
            let degree = item
                .get("degree")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("education.{}", slugify(degree))
        },
    )?;

    import_list(
        conn,
        &now,
        &project_root.join("content/experience.yaml"),
        "experience",
        ElementKind::Experience,
        options,
        &mut stats,
        |item| {
            let company = item
                .get("company")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let slug_suffix = element_date_slug(item);
            format!("experience.{}-{}", slugify(company), slug_suffix)
        },
    )?;

    import_list(
        conn,
        &now,
        &project_root.join("content/projects.yaml"),
        "projects",
        ElementKind::Project,
        options,
        &mut stats,
        |item| {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            format!("project.{}", slugify(name))
        },
    )?;

    Ok(stats)
}

fn import_singleton(
    conn: &Connection,
    now: &str,
    path: &Path,
    kind: ElementKind,
    element_id: &str,
    sort_order: i32,
    options: ImportOptions,
    stats: &mut ImportStats,
) -> Result<()> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value: Value =
        serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    import_element(conn, now, element_id, kind, sort_order, value, options, stats)?;
    Ok(())
}

fn import_list<F>(
    conn: &Connection,
    now: &str,
    path: &Path,
    list_key: &str,
    kind: ElementKind,
    options: ImportOptions,
    stats: &mut ImportStats,
    id_for: F,
) -> Result<()>
where
    F: Fn(&Value) -> String,
{
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let root: Value =
        serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    let Some(items) = root.get(list_key).and_then(|v| v.as_array()) else {
        anyhow::bail!("expected array at key `{list_key}` in {}", path.display());
    };

    for (index, item) in items.iter().enumerate() {
        let element_id = id_for(item);
        import_element(
            conn,
            now,
            &element_id,
            kind,
            index as i32,
            item.clone(),
            options,
            stats,
        )?;
    }
    Ok(())
}

fn import_element(
    conn: &Connection,
    now: &str,
    element_id: &str,
    kind: ElementKind,
    sort_order: i32,
    payload: Value,
    options: ImportOptions,
    stats: &mut ImportStats,
) -> Result<()> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM elements WHERE id = ?1",
        [element_id],
        |row| row.get(0),
    )?;

    let mut payload = payload;
    if matches!(
        kind,
        ElementKind::Education | ElementKind::Experience | ElementKind::Project
    ) {
        domain::normalize_payload_dates(&mut payload)
            .map_err(|e| anyhow::anyhow!("normalize dates for `{element_id}`: {e}"))?;
    }
    if kind == ElementKind::Experience {
        domain::normalize_experience_location(&mut payload);
    }

    if exists > 0 {
        if options.force {
            domain::validate_payload(kind, &payload)?;
            append_revision(conn, now, element_id, payload)?;
            stats.appended += 1;
        } else {
            stats.skipped += 1;
        }
        return Ok(());
    }

    domain::validate_payload(kind, &payload)?;

    conn.execute(
        "INSERT INTO elements (id, kind, created_at, sort_order) VALUES (?1, ?2, ?3, ?4)",
        params![element_id, kind.as_str(), now, sort_order],
    )?;

    insert_revision(conn, now, element_id, 1, payload)?;
    stats.created += 1;
    Ok(())
}

fn insert_revision(
    conn: &Connection,
    now: &str,
    element_id: &str,
    rev_number: i64,
    payload: Value,
) -> Result<()> {
    let rev_id = Uuid::new_v4().to_string();
    let payload_json = serde_json::to_string(&payload)?;
    conn.execute(
        "INSERT INTO element_revisions (id, element_id, rev_number, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![rev_id, element_id, rev_number, payload_json, now],
    )?;
    Ok(())
}

fn append_revision(
    conn: &Connection,
    now: &str,
    element_id: &str,
    payload: Value,
) -> Result<()> {
    let next_rev: i64 = conn.query_row(
        "SELECT COALESCE(MAX(rev_number), 0) + 1 FROM element_revisions WHERE element_id = ?1",
        [element_id],
        |row| row.get(0),
    )?;
    insert_revision(conn, now, element_id, next_rev, payload)
}

fn element_date_slug(item: &Value) -> String {
    if let Some(s) = item.get("date").and_then(|v| v.as_str()) {
        return slugify(s);
    }
    if let Some(dr) = item.get("date_range") {
        if let Ok(range) = serde_json::from_value::<domain::DateRange>(dr.clone()) {
            let start = format!("{}-{}", range.start.year, range.start.month);
            let end = match range.end {
                domain::DateRangeEnd::Present => "present".to_string(),
                domain::DateRangeEnd::Date { date } => format!("{}-{}", date.year, date.month),
            };
            return slugify(&format!("{start}-{end}"));
        }
    }
    String::new()
}

fn slugify(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut out = String::new();
    let mut last_dash = false;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}
