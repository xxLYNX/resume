use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Composition;

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Immutable snapshot written at submit time (schema v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionManifest {
    pub schema_version: u32,
    pub submitted_at: String,
    pub theme_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_files_hash: Option<String>,
    pub composition: Composition,
    /// Denormalized MiniJinja contexts per section key (enough to re-render).
    pub resolved: BTreeMap<String, Value>,
    /// Hash of theme + composition + resolved (stable for draft-vs-submitted warnings).
    pub snapshot_hash: String,
    pub content_hash: String,
}

impl SubmissionManifest {
    pub fn new(
        submitted_at: String,
        theme_id: String,
        theme_files_hash: Option<String>,
        composition: Composition,
        resolved: BTreeMap<String, Value>,
    ) -> Self {
        let snapshot_hash =
            compute_snapshot_hash(&theme_id, theme_files_hash.as_deref(), &composition, &resolved);
        let mut manifest = Self {
            schema_version: MANIFEST_SCHEMA_VERSION,
            submitted_at,
            theme_id,
            theme_files_hash,
            composition,
            resolved,
            snapshot_hash,
            content_hash: String::new(),
        };
        manifest.content_hash = compute_content_hash(&manifest);
        manifest
    }
}

pub fn compute_snapshot_hash(
    theme_id: &str,
    theme_files_hash: Option<&str>,
    composition: &Composition,
    resolved: &BTreeMap<String, Value>,
) -> String {
    let body = SnapshotHashBody {
        theme_id,
        theme_files_hash,
        composition,
        resolved,
    };
    hash_json_body(&body)
}

/// SHA-256 of canonical JSON (sorted map keys) excluding `content_hash`.
pub fn compute_content_hash(manifest: &SubmissionManifest) -> String {
    let body = ManifestHashBody {
        schema_version: manifest.schema_version,
        submitted_at: manifest.submitted_at.as_str(),
        snapshot_hash: manifest.snapshot_hash.as_str(),
    };
    hash_json_body(&body)
}

fn hash_json_body<T: Serialize>(body: &T) -> String {
    let json = serde_json::to_string(body).expect("hash body serializes");
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[derive(Serialize)]
struct ManifestHashBody<'a> {
    schema_version: u32,
    submitted_at: &'a str,
    snapshot_hash: &'a str,
}

#[derive(Serialize)]
struct SnapshotHashBody<'a> {
    theme_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    theme_files_hash: Option<&'a str>,
    composition: &'a Composition,
    resolved: &'a BTreeMap<String, Value>,
}
