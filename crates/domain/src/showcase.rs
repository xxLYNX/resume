use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use std::fmt::{self, Display, Formatter};

use crate::{Composition, CompositionSection};

/// Optimistic concurrency or in-flight approval race (maps to HTTP 409).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowcaseConflict;

impl Display for ShowcaseConflict {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "showcase conflict")
    }
}

impl std::error::Error for ShowcaseConflict {}

/// Singleton row id. One Showcase per workspace.
pub const SHOWCASE_ID: &str = "default";

/// Initial draft theme. Approval freezes whatever theme is on the draft at that moment.
pub const SHOWCASE_DEFAULT_THEME_ID: &str = "classic";

/// Section keys for an empty Showcase draft. Items are not filled from the catalog.
pub const SHOWCASE_SECTION_KEYS: &[&str] = &[
    "header",
    "objective",
    "education",
    "experience",
    "projects",
    "certifications",
    "awards",
];

/// First Showcase draft: the theme's sections and no pinned elements.
pub fn empty_showcase_composition() -> Composition {
    Composition {
        sections: SHOWCASE_SECTION_KEYS
            .iter()
            .map(|section_key| CompositionSection {
                section_key: (*section_key).to_string(),
                items: Vec::new(),
            })
            .collect(),
    }
}

/// Editable Showcase draft. Not an application and not a preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowcaseDraft {
    pub theme_id: String,
    pub composition: Composition,
    pub current_publication_id: Option<String>,
    pub updated_at: String,
}

/// Immutable approved snapshot. `resolved` is copied at approval and does not follow the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowcasePublication {
    pub id: String,
    pub created_at: String,
    pub theme_id: String,
    pub theme_files_hash: Option<String>,
    pub composition: Composition,
    pub resolved: BTreeMap<String, Value>,
    pub snapshot_hash: String,
    pub pdf_relpath: String,
    pub pdf_sha256: String,
}

pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

/// Publication identity for one rendered draft. Includes the theme-file hash and the
/// enabled section names taken from the build config. `settings/jobdesk.yaml` output
/// filename and comments are not inputs; section enablement is.
pub fn compute_showcase_snapshot_hash(
    theme_id: &str,
    theme_files_hash: &str,
    enabled_sections: &[String],
    composition: &Composition,
    resolved: &BTreeMap<String, Value>,
) -> String {
    let body = ShowcaseSnapshotBody {
        theme_id,
        theme_files_hash,
        enabled_sections,
        composition,
        resolved,
    };
    let json = serde_json::to_string(&body).expect("showcase snapshot hash body serializes");
    sha256_prefixed(json.as_bytes())
}

pub fn showcase_composition_is_empty(composition: &Composition) -> bool {
    composition
        .sections
        .iter()
        .all(|section| section.items.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_hash_changes_when_enabled_sections_change() {
        let composition = empty_showcase_composition();
        let resolved = BTreeMap::new();
        let header_only = compute_showcase_snapshot_hash(
            "classic",
            "sha256:theme",
            &["header".into()],
            &composition,
            &resolved,
        );
        let with_experience = compute_showcase_snapshot_hash(
            "classic",
            "sha256:theme",
            &["header".into(), "experience".into()],
            &composition,
            &resolved,
        );
        assert_ne!(header_only, with_experience);
    }
}

#[derive(Serialize)]
struct ShowcaseSnapshotBody<'a> {
    theme_id: &'a str,
    theme_files_hash: &'a str,
    enabled_sections: &'a [String],
    composition: &'a Composition,
    resolved: &'a BTreeMap<String, Value>,
}
