use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{Context, Result};
use domain::{
    validate_revision_links, validate_skill_node, Composition, RevisionSkillLink, SkillKind,
    SkillNode,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::Store;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillLinkRow {
    pub skill_id: String,
    pub axis: String,
    pub source: String,
    pub label: String,
}

impl Store {
    pub fn list_skill_nodes(&self, kind: Option<&str>) -> Result<Vec<SkillNode>> {
        let conn = self.connection();
        let (sql, filter): (&str, bool) = match kind {
            Some(_k) => (
                "SELECT id, kind, label, parent_id, aliases_json, pdf_groups_json, framework_refs_json
                 FROM skill_nodes WHERE kind = ?1 ORDER BY label COLLATE NOCASE",
                true,
            ),
            None => (
                "SELECT id, kind, label, parent_id, aliases_json, pdf_groups_json, framework_refs_json
                 FROM skill_nodes ORDER BY kind, label COLLATE NOCASE",
                false,
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = if filter {
            stmt.query_map([kind.unwrap()], map_skill_node)?
        } else {
            stmt.query_map([], map_skill_node)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_skill_node(&self, id: &str) -> Result<Option<SkillNode>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT id, kind, label, parent_id, aliases_json, pdf_groups_json, framework_refs_json
             FROM skill_nodes WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], map_skill_node)?;
        Ok(rows.next().transpose()?)
    }

    pub fn upsert_skill_node(&self, node: &SkillNode) -> Result<()> {
        validate_skill_node(node).map_err(|e| anyhow::anyhow!("{e}"))?;
        let conn = self.connection();
        let aliases = serde_json::to_string(&node.aliases)?;
        let pdf_groups = serde_json::to_string(&node.pdf_groups)?;
        let framework_refs = if node.framework_refs.is_null() {
            "{}".to_string()
        } else {
            node.framework_refs.to_string()
        };
        conn.execute(
            "INSERT INTO skill_nodes (id, kind, label, parent_id, aliases_json, pdf_groups_json, framework_refs_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               kind = excluded.kind,
               label = excluded.label,
               parent_id = excluded.parent_id,
               aliases_json = excluded.aliases_json,
               pdf_groups_json = excluded.pdf_groups_json,
               framework_refs_json = excluded.framework_refs_json",
            params![
                node.id,
                node.kind.as_str(),
                node.label,
                node.parent_id,
                aliases,
                pdf_groups,
                framework_refs,
            ],
        )?;
        Ok(())
    }

    pub fn import_skill_nodes(&self, nodes: &[SkillNode]) -> Result<usize> {
        let mut n = 0;
        for node in nodes {
            self.upsert_skill_node(node)?;
            n += 1;
        }
        Ok(n)
    }

    pub fn get_revision_skill_links(&self, revision_id: &str) -> Result<Vec<SkillLinkRow>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT l.skill_id, l.axis, l.source, n.label
             FROM revision_skill_links l
             JOIN skill_nodes n ON n.id = l.skill_id
             WHERE l.revision_id = ?1
             ORDER BY l.axis, n.label COLLATE NOCASE",
        )?;
        let rows = stmt
            .query_map([revision_id], |row| {
                Ok(SkillLinkRow {
                    skill_id: row.get(0)?,
                    axis: row.get(1)?,
                    source: row.get(2)?,
                    label: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_revision_skill_links(
        &self,
        revision_id: &str,
        links: &[RevisionSkillLink],
    ) -> Result<()> {
        let node_kind = |id: &str| {
            self.get_skill_node(id)
                .ok()
                .flatten()
                .map(|n| n.kind)
        };
        validate_revision_links(links, |id| node_kind(id))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let conn = self.connection();
        conn.execute(
            "DELETE FROM revision_skill_links WHERE revision_id = ?1 AND source = 'asserted'",
            [revision_id],
        )?;
        for link in links {
            if link.source != "asserted" {
                continue;
            }
            conn.execute(
                "INSERT INTO revision_skill_links (revision_id, skill_id, axis, source)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    revision_id,
                    link.skill_id,
                    link.axis.as_str(),
                    link.source,
                ],
            )?;
        }
        Ok(())
    }

    /// Aggregate asserted skill labels from experience/project pins into classic PDF groups.
    pub fn aggregate_skills_pdf_context(&self, composition: &Composition) -> Result<Value> {
        let mut by_group: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for section in &composition.sections {
            if section.section_key != "experience" && section.section_key != "projects" {
                continue;
            }
            for item in &section.items {
                let links = self.get_revision_skill_links(&item.revision_id)?;
                for row in links {
                    if row.source != "asserted" {
                        continue;
                    }
                    let node = self
                        .get_skill_node(&row.skill_id)?
                        .with_context(|| format!("missing skill `{}`", row.skill_id))?;
                    let groups = if node.pdf_groups.is_empty() {
                        vec!["other".to_string()]
                    } else {
                        node.pdf_groups.clone()
                    };
                    for g in groups {
                        by_group
                            .entry(g)
                            .or_default()
                            .insert(node.label.clone());
                    }
                }
            }
        }

        let mut skills_obj = serde_json::Map::new();
        for (group, labels) in by_group {
            let joined = labels.into_iter().collect::<Vec<_>>().join(", ");
            skills_obj.insert(group, Value::String(joined));
        }
        Ok(json!({ "skills": Value::Object(skills_obj) }))
    }
}

fn map_skill_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillNode> {
    let kind_str: String = row.get(1)?;
    let kind = SkillKind::parse(&kind_str).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(1, kind_str, rusqlite::types::Type::Text)
    })?;
    let aliases: Vec<String> = serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default();
    let pdf_groups: Vec<String> =
        serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default();
    let framework_refs: Value =
        serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(json!({}));
    Ok(SkillNode {
        id: row.get(0)?,
        kind,
        label: row.get(2)?,
        parent_id: row.get(3)?,
        aliases,
        pdf_groups,
        framework_refs,
    })
}

pub fn seed_skill_nodes_if_empty(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM skill_nodes", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let raw = include_str!("../../../data/skills_seed.json");
    let nodes: Vec<SkillNode> = serde_json::from_str(raw).context("parse embedded skills_seed")?;
    for node in nodes {
        let aliases = serde_json::to_string(&node.aliases)?;
        let pdf_groups = serde_json::to_string(&node.pdf_groups)?;
        let framework_refs = if node.framework_refs.is_null() {
            "{}".to_string()
        } else {
            node.framework_refs.to_string()
        };
        conn.execute(
            "INSERT INTO skill_nodes (id, kind, label, parent_id, aliases_json, pdf_groups_json, framework_refs_json)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)",
            params![
                node.id,
                node.kind.as_str(),
                node.label,
                aliases,
                pdf_groups,
                framework_refs,
            ],
        )?;
    }
    Ok(())
}

/// Merge aggregated link labels into an existing skills render context.
pub fn merge_skills_context(base: Value, aggregated: Value) -> Value {
    let Some(base_skills) = base.get("skills").and_then(|v| v.as_object()) else {
        return aggregated;
    };
    let Some(agg_skills) = aggregated.get("skills").and_then(|v| v.as_object()) else {
        return base;
    };

    let mut merged: HashMap<String, BTreeSet<String>> = HashMap::new();

    for (key, val) in base_skills {
        if let Some(s) = val.as_str() {
            for part in split_skill_line(s) {
                merged.entry(key.clone()).or_default().insert(part);
            }
        }
    }
    for (key, val) in agg_skills {
        if let Some(s) = val.as_str() {
            for part in split_skill_line(s) {
                merged.entry(key.clone()).or_default().insert(part);
            }
        }
    }

    let mut out_skills = serde_json::Map::new();
    for (key, set) in merged {
        let line = set.into_iter().collect::<Vec<_>>().join(", ");
        if !line.is_empty() {
            out_skills.insert(key, Value::String(line));
        }
    }
    json!({ "skills": Value::Object(out_skills) })
}

fn split_skill_line(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}
