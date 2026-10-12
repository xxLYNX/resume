use std::collections::BTreeSet;

use anyhow::{Context, Result};
use domain::{
    validate_element_links, validate_skill_node, Composition, ElementSkillLink, SkillKind,
    SkillNode,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::Store;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillLinkRow {
    pub skill_id: String,
    pub kind: String,
    pub source: String,
    pub label: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompetencySuggestion {
    pub competency_id: String,
    pub label: String,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct ListSkillRelationshipsFilter {
    pub skill_id: Option<String>,
    pub element_id: Option<String>,
    pub skill_kind: Option<String>,
    pub element_kind: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillRelationshipRow {
    pub element_id: String,
    pub element_kind: String,
    pub skill_id: String,
    pub skill_kind: String,
    pub skill_label: String,
    pub source: String,
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

    pub fn get_element_skill_links(&self, element_id: &str) -> Result<Vec<SkillLinkRow>> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT l.skill_id, n.kind, l.source, n.label
             FROM element_skill_links l
             JOIN skill_nodes n ON n.id = l.skill_id
             WHERE l.element_id = ?1
             ORDER BY n.kind, n.label COLLATE NOCASE",
        )?;
        let rows = stmt
            .query_map([element_id], |row| {
                Ok(SkillLinkRow {
                    skill_id: row.get(0)?,
                    kind: row.get(1)?,
                    source: row.get(2)?,
                    label: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// All element ↔ skill evidence links (same junction as resolver aggregation).
    pub fn list_skill_relationships(
        &self,
        filter: &ListSkillRelationshipsFilter,
    ) -> Result<Vec<SkillRelationshipRow>> {
        let conn = self.connection();
        let mut sql = String::from(
            "SELECT e.id, e.kind, l.skill_id, n.kind, n.label, l.source
             FROM element_skill_links l
             JOIN elements e ON e.id = l.element_id
             JOIN skill_nodes n ON n.id = l.skill_id
             WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(id) = filter.element_id.as_deref().filter(|s| !s.is_empty()) {
            sql.push_str(" AND e.id = ?");
            params.push(Box::new(id.to_string()));
        }
        if let Some(id) = filter.skill_id.as_deref().filter(|s| !s.is_empty()) {
            sql.push_str(" AND l.skill_id = ?");
            params.push(Box::new(id.to_string()));
        }
        if let Some(kind) = filter.element_kind.as_deref().filter(|s| !s.is_empty()) {
            sql.push_str(" AND e.kind = ?");
            params.push(Box::new(kind.to_string()));
        }
        if let Some(kind) = filter.skill_kind.as_deref().filter(|s| !s.is_empty()) {
            sql.push_str(" AND n.kind = ?");
            params.push(Box::new(kind.to_string()));
        }
        sql.push_str(" ORDER BY n.label COLLATE NOCASE, e.kind, e.id");

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(SkillRelationshipRow {
                    element_id: row.get(0)?,
                    element_kind: row.get(1)?,
                    skill_id: row.get(2)?,
                    skill_kind: row.get(3)?,
                    skill_label: row.get(4)?,
                    source: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_element_skill_links(
        &self,
        element_id: &str,
        links: &[ElementSkillLink],
    ) -> Result<()> {
        let node_kind = |id: &str| {
            self.get_skill_node(id)
                .ok()
                .flatten()
                .map(|n| n.kind)
        };
        validate_element_links(links, |id| node_kind(id))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let conn = self.connection();
        conn.execute(
            "DELETE FROM element_skill_links WHERE element_id = ?1 AND source = 'asserted'",
            [element_id],
        )?;
        for link in links {
            if link.source != "asserted" {
                continue;
            }
            conn.execute(
                "INSERT INTO element_skill_links (element_id, skill_id, source)
                 VALUES (?1, ?2, ?3)",
                params![element_id, link.skill_id, link.source],
            )?;
        }
        Ok(())
    }

    pub fn suggest_competencies_for_tools(&self, tool_ids: &[String]) -> Result<Vec<CompetencySuggestion>> {
        if tool_ids.is_empty() {
            return Ok(vec![]);
        }
        let conn = self.connection();
        let placeholders = tool_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT DISTINCT e.competency_id, n.label, e.source
             FROM tool_competency_edges e
             JOIN skill_nodes n ON n.id = e.competency_id
             WHERE e.tool_id IN ({placeholders})
             ORDER BY n.label COLLATE NOCASE"
        );
        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = tool_ids
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok(CompetencySuggestion {
                    competency_id: row.get(0)?,
                    label: row.get(1)?,
                    source: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Deduped catalog labels from asserted links on pinned evidence elements (all kinds).
    pub fn aggregate_toolbelt_tags(&self, composition: &Composition) -> Result<Vec<String>> {
        const EVIDENCE_SECTIONS: &[&str] = &[
            "experience",
            "projects",
            "education",
            "certifications",
            "awards",
        ];
        let mut labels: BTreeSet<String> = BTreeSet::new();
        let mut seen_elements: BTreeSet<String> = BTreeSet::new();

        for section in &composition.sections {
            if !EVIDENCE_SECTIONS.contains(&section.section_key.as_str()) {
                continue;
            }
            for item in &section.items {
                if !seen_elements.insert(item.element_id.clone()) {
                    continue;
                }
                self.collect_asserted_skill_labels(&item.element_id, &mut labels)?;
            }
        }

        self.collect_course_skill_labels(&seen_elements, &mut labels)?;
        Ok(labels.into_iter().collect())
    }

    fn collect_asserted_skill_labels(
        &self,
        element_id: &str,
        labels: &mut BTreeSet<String>,
    ) -> Result<()> {
        let links = self.get_element_skill_links(element_id)?;
        for row in links {
            if row.source != "asserted" {
                continue;
            }
            let node = self
                .get_skill_node(&row.skill_id)?
                .with_context(|| format!("missing skill `{}`", row.skill_id))?;
            labels.insert(node.label.clone());
        }
        Ok(())
    }

    fn collect_course_skill_labels(
        &self,
        pinned_elements: &BTreeSet<String>,
        labels: &mut BTreeSet<String>,
    ) -> Result<()> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT id, parent_element_id FROM elements WHERE kind = 'course' ORDER BY sort_order, id",
        )?;
        let courses: Vec<(String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        for (element_id, parent_id) in courses {
            let include = pinned_elements.contains(&element_id)
                || parent_id
                    .as_ref()
                    .is_some_and(|p| pinned_elements.contains(p));
            if !include {
                continue;
            }
            self.collect_asserted_skill_labels(&element_id, labels)?;
        }
        Ok(())
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
    upsert_catalog_seed(conn)
}

/// Merge bundled catalog nodes (insert or update by id). Used on fresh DB and migration v12+.
pub fn upsert_catalog_seed(conn: &Connection) -> Result<()> {
    let raw = include_str!("../../../data/catalog_seed.json");
    let nodes: Vec<SkillNode> = serde_json::from_str(raw).context("parse catalog_seed")?;
    for node in nodes {
        validate_skill_node(&node)?;
        let aliases = serde_json::to_string(&node.aliases)?;
        let pdf_groups = serde_json::to_string(&node.pdf_groups)?;
        let framework_refs = if node.framework_refs.is_null() {
            "{}".to_string()
        } else {
            node.framework_refs.to_string()
        };
        conn.execute(
            "INSERT INTO skill_nodes (id, kind, label, parent_id, aliases_json, pdf_groups_json, framework_refs_json)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               kind = excluded.kind,
               label = excluded.label,
               aliases_json = excluded.aliases_json,
               pdf_groups_json = excluded.pdf_groups_json,
               framework_refs_json = excluded.framework_refs_json",
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

pub fn seed_tool_competency_edges(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM tool_competency_edges", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    #[derive(serde::Deserialize)]
    struct Edge {
        tool_id: String,
        competency_id: String,
    }
    let raw = include_str!("../../../data/tool_competency_edges.json");
    let edges: Vec<Edge> = serde_json::from_str(raw).context("parse tool_competency_edges")?;
    for edge in edges {
        conn.execute(
            "INSERT OR IGNORE INTO tool_competency_edges (tool_id, competency_id, source)
             VALUES (?1, ?2, 'import')",
            params![edge.tool_id, edge.competency_id],
        )?;
    }
    Ok(())
}
