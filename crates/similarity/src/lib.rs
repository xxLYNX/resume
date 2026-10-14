use std::collections::HashSet;

use domain::Composition;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct RankInput<'a> {
    pub id: &'a str,
    pub company: &'a str,
    pub title: &'a str,
    pub tags: &'a [String],
    pub composition: &'a Composition,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RankedMatch {
    pub id: String,
    pub company: String,
    pub title: String,
    /// Combined score in [0, 1].
    pub score: f64,
    /// Rounded element-id Jaccard × 100 for UI hints.
    pub overlap_percent: u8,
}

/// Rank candidates by composition overlap, tag overlap, and title token overlap.
pub fn rank_similar(source: &RankInput<'_>, candidates: &[RankInput<'_>], limit: usize) -> Vec<RankedMatch> {
    let source_elements = element_ids(source.composition);
    let source_tags: HashSet<&str> = source.tags.iter().map(String::as_str).collect();
    let source_title = title_tokens(source.title);

    let mut scored: Vec<RankedMatch> = candidates
        .iter()
        .filter(|c| c.id != source.id)
        .map(|c| {
            let elements = element_ids(c.composition);
            let element_j = jaccard(&source_elements, &elements);
            let tags: HashSet<&str> = c.tags.iter().map(String::as_str).collect();
            let tag_j = jaccard(&source_tags, &tags);
            let title_j = jaccard(&source_title, &title_tokens(c.title));
            let score = 0.55 * element_j + 0.30 * tag_j + 0.15 * title_j;
            RankedMatch {
                id: c.id.to_string(),
                company: c.company.to_string(),
                title: c.title.to_string(),
                score,
                overlap_percent: (element_j * 100.0).round().clamp(0.0, 100.0) as u8,
            }
        })
        .filter(|m| m.score > 0.0)
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(limit);
    scored
}

fn element_ids(composition: &Composition) -> HashSet<String> {
    composition
        .sections
        .iter()
        .flat_map(|s| s.items.iter().map(|i| i.element_id.clone()))
        .collect()
}

fn title_tokens(title: &str) -> HashSet<String> {
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(str::to_string)
        .collect()
}

fn jaccard<T: Eq + std::hash::Hash>(a: &HashSet<T>, b: &HashSet<T>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{CompositionItem, CompositionSection};

    fn comp(ids: &[&str]) -> Composition {
        Composition {
            sections: vec![CompositionSection {
                section_key: "experience".into(),
                items: ids
                    .iter()
                    .map(|id| CompositionItem {
                        element_id: (*id).into(),
                        revision_id: "rev".into(),
                        resume_tag_skill_ids: Vec::new(),
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn ranks_by_element_overlap() {
        let a = RankInput {
            id: "a",
            company: "A",
            title: "Security Engineer",
            tags: &["sec".into()],
            composition: &comp(&["e1", "e2"]),
        };
        let b = RankInput {
            id: "b",
            company: "B",
            title: "Security Analyst",
            tags: &["sec".into()],
            composition: &comp(&["e1", "e2", "e3"]),
        };
        let c = RankInput {
            id: "c",
            company: "C",
            title: "Chef",
            tags: &[],
            composition: &comp(&["x"]),
        };
        let ranked = rank_similar(&a, &[b, c], 2);
        assert_eq!(ranked[0].id, "b");
        assert!(ranked[0].score > ranked.get(1).map(|m| m.score).unwrap_or(0.0));
    }
}
