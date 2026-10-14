use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use domain::{SkillKind, SkillNode};
use serde_json::json;

use crate::slug::{normalize_match_key, slugify_label};
use crate::OnetImportResult;

const MAX_LABEL_LEN: usize = 80;

#[derive(Debug, Clone)]
pub struct ToolRow {
    pub soc: String,
    pub example: String,
    pub commodity_code: String,
    pub commodity_title: String,
    pub hot_technology: bool,
    pub in_demand: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolCompetencyEdge {
    pub tool_id: String,
    pub competency_id: String,
}

#[derive(Debug, Clone)]
struct SkillRatingRow {
    soc: String,
    element_id: String,
    element_name: String,
    scale_id: String,
    data_value: f64,
    not_relevant: bool,
}

pub fn read_technology_skills_file(path: &Path) -> Result<Vec<ToolRow>> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut rows = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if i == 0 && line.starts_with("O*NET-SOC") {
            continue;
        }
        let cols = split_onet_line(&line);
        if cols.len() < 6 {
            continue;
        }
        let example = cols[1].trim();
        if example.is_empty() {
            continue;
        }
        rows.push(ToolRow {
            soc: cols[0].trim().to_string(),
            example: example.to_string(),
            commodity_code: cols[2].trim().to_string(),
            commodity_title: cols[3].trim().to_string(),
            hot_technology: cols[4].trim().eq_ignore_ascii_case("y"),
            in_demand: cols[5].trim().eq_ignore_ascii_case("y"),
        });
    }
    Ok(rows)
}

fn read_skills_ratings_file(path: &Path) -> Result<Vec<SkillRatingRow>> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut rows = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if i == 0 && line.starts_with("O*NET-SOC") {
            continue;
        }
        let cols = split_onet_line(&line);
        if cols.len() < 13 {
            continue;
        }
        let data_value: f64 = cols[4].trim().parse().unwrap_or(0.0);
        let not_relevant = cols[10].trim().eq_ignore_ascii_case("y");
        rows.push(SkillRatingRow {
            soc: cols[0].trim().to_string(),
            element_id: cols[1].trim().to_string(),
            element_name: cols[2].trim().to_string(),
            scale_id: cols[3].trim().to_string(),
            data_value,
            not_relevant,
        });
    }
    Ok(rows)
}

fn split_onet_line(line: &str) -> Vec<String> {
    if line.contains('\t') {
        return line.split('\t').map(|s| s.to_string()).collect();
    }
    line.split_whitespace().map(|s| s.to_string()).collect()
}

pub fn build_tool_nodes(rows: &[ToolRow]) -> Vec<SkillNode> {
    let mut by_key: BTreeMap<String, ToolAggregate> = BTreeMap::new();
    for row in rows {
        let key = normalize_match_key(&row.example);
        if key.is_empty() {
            continue;
        }
        by_key
            .entry(key)
            .or_insert_with(|| ToolAggregate::new(&row.example))
            .merge(row);
    }

    let mut nodes: Vec<SkillNode> = by_key
        .into_values()
        .map(|agg| agg.into_node())
        .collect();
    nodes.sort_by(|a, b| a.label.cmp(&b.label));
    nodes
}

struct ToolAggregate {
    example: String,
    aliases: BTreeSet<String>,
    commodity_titles: BTreeSet<String>,
    socs: BTreeSet<String>,
    hot: bool,
    in_demand: bool,
}

impl ToolAggregate {
    fn new(example: &str) -> Self {
        Self {
            example: example.trim().to_string(),
            aliases: BTreeSet::new(),
            commodity_titles: BTreeSet::new(),
            socs: BTreeSet::new(),
            hot: false,
            in_demand: false,
        }
    }

    fn merge(&mut self, row: &ToolRow) {
        if row.example.len() > self.example.len() {
            self.aliases.insert(normalize_display(&self.example));
            self.example = row.example.trim().to_string();
        } else if row.example.trim() != self.example {
            self.aliases.insert(normalize_display(&row.example));
        }
        if !row.commodity_title.is_empty() {
            self.commodity_titles.insert(row.commodity_title.trim().to_string());
        }
        self.socs.insert(row.soc.clone());
        self.hot |= row.hot_technology;
        self.in_demand |= row.in_demand;
    }

    fn into_node(self) -> SkillNode {
        let id = format!("tool.{}", slugify_label(&self.example));
        let label = truncate_label(&self.example);
        let mut aliases: Vec<String> = self
            .aliases
            .into_iter()
            .chain(self.commodity_titles)
            .filter(|a| !a.eq_ignore_ascii_case(&label))
            .take(20)
            .collect();
        aliases.sort_by_key(|a| a.to_lowercase());
        aliases.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

        SkillNode {
            id,
            kind: SkillKind::Tool,
            label,
            parent_id: None,
            aliases,
            pdf_groups: vec![],
            framework_refs: json!({
                "onet": {
                    "source": "technology_skills",
                    "example": self.example,
                    "hot_technology": self.hot,
                    "in_demand": self.in_demand,
                    "sample_socs": self.socs.iter().take(5).collect::<Vec<_>>(),
                }
            }),
        }
    }
}

fn build_competency_nodes(ratings: &[SkillRatingRow]) -> Vec<SkillNode> {
    let mut by_element: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
    for row in ratings {
        if row.element_id.is_empty() || row.element_name.is_empty() {
            continue;
        }
        by_element
            .entry(row.element_id.clone())
            .or_insert_with(|| (row.element_name.clone(), BTreeSet::new()))
            .1
            .insert(row.soc.clone());
    }

    let mut nodes = Vec::new();
    for (element_id, (element_name, socs)) in by_element {
        let id = format!(
            "competency.onet.{}",
            slugify_label(&element_id.replace('.', "-"))
        );
        let label = truncate_label(&element_name);
        nodes.push(SkillNode {
            id,
            kind: SkillKind::Competency,
            label,
            parent_id: None,
            aliases: vec![],
            pdf_groups: vec![],
            framework_refs: json!({
                "onet": {
                    "source": "skills",
                    "element_id": element_id,
                    "element_name": element_name,
                    "sample_socs": socs.iter().take(5).collect::<Vec<_>>(),
                }
            }),
        });
    }
    nodes.sort_by(|a, b| a.label.cmp(&b.label));
    nodes
}

fn build_tool_competency_edges(
    tool_rows: &[ToolRow],
    skill_ratings: &[SkillRatingRow],
    importance_threshold: f64,
) -> Vec<ToolCompetencyEdge> {
    let mut tools_by_soc: HashMap<String, BTreeSet<String>> = HashMap::new();
    for row in tool_rows {
        let key = normalize_match_key(&row.example);
        if key.is_empty() {
            continue;
        }
        let tool_id = format!("tool.{}", slugify_label(&row.example));
        tools_by_soc
            .entry(row.soc.clone())
            .or_default()
            .insert(tool_id);
    }

    let mut comps_by_soc: HashMap<String, BTreeSet<String>> = HashMap::new();
    for row in skill_ratings {
        if row.not_relevant || row.scale_id != "IM" {
            continue;
        }
        if row.data_value < importance_threshold {
            continue;
        }
        let comp_id = format!(
            "competency.onet.{}",
            slugify_label(&row.element_id.replace('.', "-"))
        );
        comps_by_soc
            .entry(row.soc.clone())
            .or_default()
            .insert(comp_id);
    }

    let mut edge_set: BTreeSet<ToolCompetencyEdge> = BTreeSet::new();
    for (soc, tools) in &tools_by_soc {
        let Some(comps) = comps_by_soc.get(soc) else {
            continue;
        };
        for tool_id in tools {
            for competency_id in comps {
                edge_set.insert(ToolCompetencyEdge {
                    tool_id: tool_id.clone(),
                    competency_id: competency_id.clone(),
                });
            }
        }
    }
    edge_set.into_iter().collect()
}

fn truncate_label(s: &str) -> String {
    let t = s.trim();
    if t.len() <= MAX_LABEL_LEN {
        return t.to_string();
    }
    let mut end = MAX_LABEL_LEN;
    while end > 0 && !t.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &t[..end])
}

fn normalize_display(s: &str) -> String {
    s.trim().to_string()
}

pub fn import_from_onet_dir(dir: &Path) -> Result<OnetImportResult> {
    let tech_path = dir.join("Technology Skills.txt");
    let software_path = dir.join("Software Skills.txt");
    let skills_path = dir.join("Skills.txt");

    let mut tool_rows = Vec::new();
    if tech_path.is_file() {
        tool_rows.extend(read_technology_skills_file(&tech_path)?);
    }
    if software_path.is_file() {
        tool_rows.extend(read_technology_skills_file(&software_path)?);
    }
    if tool_rows.is_empty() {
        anyhow::bail!(
            "no tool rows found; expected {:?} and/or {:?}",
            tech_path,
            software_path
        );
    }

    let skill_ratings = read_skills_ratings_file(&skills_path)
        .with_context(|| format!("read {}", skills_path.display()))?;

    let tools = build_tool_nodes(&tool_rows);
    let competencies = build_competency_nodes(&skill_ratings);
    let edges = build_tool_competency_edges(
        &tool_rows,
        &skill_ratings,
        crate::DEFAULT_COMPETENCY_IMPORTANCE_THRESHOLD,
    );

    Ok(OnetImportResult {
        tools,
        competencies,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/onet")
            .join(name)
    }

    #[test]
    fn dedupes_tools_by_example() {
        let rows = read_technology_skills_file(&fixture("Technology Skills.txt")).unwrap();
        let nodes = build_tool_nodes(&rows);
        assert_eq!(nodes.len(), 3);
        assert!(nodes.iter().any(|n| n.label == "Python"));
        assert_eq!(
            nodes.iter().filter(|n| n.label == "Python").count(),
            1,
            "duplicate Python rows merge"
        );
    }

    #[test]
    fn builds_competencies_from_element_ids() {
        let ratings = read_skills_ratings_file(&fixture("Skills.txt")).unwrap();
        let nodes = build_competency_nodes(&ratings);
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().all(|n| n.id.starts_with("competency.onet.")));
    }

    #[test]
    fn builds_edges_on_shared_soc() {
        let tool_rows = read_technology_skills_file(&fixture("Technology Skills.txt")).unwrap();
        let ratings = read_skills_ratings_file(&fixture("Skills.txt")).unwrap();
        let edges = build_tool_competency_edges(&tool_rows, &ratings, 3.0);
        assert!(!edges.is_empty());
    }
}
