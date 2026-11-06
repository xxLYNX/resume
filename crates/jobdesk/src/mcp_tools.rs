//! MCP tool handlers (testable without stdio).

use anyhow::Context;
use serde::Serialize;
use serde_json::{json, Value};
use store::{ListApplicationsQuery, Store};

const MAX_SUMMARY_TEXT_CHARS: usize = 12_000;
/// Full portfolio payloads may be large; refuse rather than truncate silently.
pub const MAX_CATALOG_JSON_BYTES: usize = 512_000;

pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "list_applications",
            "description": "List job applications in the tracker (optional q filter).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "q": { "type": "string", "description": "Search company/title" }
                }
            }
        },
        {
            "name": "get_application",
            "description": "Get one application with draft composition summary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "application_id": { "type": "string" }
                },
                "required": ["application_id"]
            }
        },
        {
            "name": "get_catalog_summary",
            "description": "List portfolio elements with truncated latest payload snippets.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Optional element kind filter" }
                }
            }
        },
        {
            "name": "get_catalog_element",
            "description": "Full portfolio element: kind, all revision metadata, latest revision payload. Local DB only — do not paste secrets into external LLM chats.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element_id": { "type": "string" }
                },
                "required": ["element_id"]
            }
        },
        {
            "name": "get_catalog_revision",
            "description": "Full payload and metadata for one catalog revision id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "revision_id": { "type": "string" }
                },
                "required": ["revision_id"]
            }
        },
        {
            "name": "get_element_usage",
            "description": "Applications and presets that reference a portfolio element id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element_id": { "type": "string" }
                },
                "required": ["element_id"]
            }
        },
        {
            "name": "get_skill_evidence",
            "description": "Asserted skill links for a portfolio element id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element_id": { "type": "string" }
                },
                "required": ["element_id"]
            }
        },
        {
            "name": "posting_hints",
            "description": "Skill/posting hint rows for an application (saved posting + draft).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "application_id": { "type": "string" }
                },
                "required": ["application_id"]
            }
        },
        {
            "name": "analyze_application",
            "description": "Full deterministic analyze report (gaps, suggestions, objective alignment).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "application_id": { "type": "string" }
                },
                "required": ["application_id"]
            }
        },
        {
            "name": "get_composition",
            "description": "Draft composition JSON for an application.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "application_id": { "type": "string" }
                },
                "required": ["application_id"]
            }
        }
    ])
}

pub fn call_tool(store: &Store, name: &str, args: Value) -> Result<Value, Value> {
    match name {
        "list_applications" => {
            let q = args.get("q").and_then(|v| v.as_str()).map(String::from);
            let apps = store
                .list_applications(&ListApplicationsQuery {
                    q,
                    phase: None,
                    status: None,
                    tag: None,
                })
                .map_err(tool_err)?;
            let rows: Vec<Value> = apps
                .iter()
                .map(|a| {
                    json!({
                        "id": a.id,
                        "company": a.company,
                        "title": a.title,
                        "phase": a.phase,
                        "status": a.status,
                        "tags": a.tags,
                    })
                })
                .collect();
            Ok(json!({ "content": [text_block(&truncate_json(&rows).map_err(tool_err)?)] }))
        }
        "get_application" => {
            let id = require_str(&args, "application_id").map_err(tool_err)?;
            let app = store
                .get_application(id)
                .map_err(tool_err)?
                .ok_or_else(|| tool_err_msg("application not found"))?;
            let posting_preview = truncate_str(&Store::application_posting_text(&app));
            let summary = json!({
                "id": app.id,
                "company": app.company,
                "title": app.title,
                "phase": app.phase,
                "status": app.status,
                "tags": app.tags,
                "posting_url": app.posting_url,
                "posting_text_chars": app.posting_text.len(),
                "posting_text_preview": posting_preview,
                "draft_section_count": app.draft_composition.sections.len(),
                "submission_count": app.submissions.len(),
            });
            Ok(json!({ "content": [text_block(&truncate_json(&summary).map_err(tool_err)?)] }))
        }
        "get_catalog_summary" => {
            let kind = args.get("kind").and_then(|v| v.as_str());
            let elements = store.list_elements(kind).map_err(tool_err)?;
            let mut rows = Vec::new();
            for el in elements.iter().take(200) {
                let snippet = store
                    .get_element(&el.id)
                    .map_err(tool_err)?
                    .and_then(|d| d.latest)
                    .map(|r| payload_snippet(&r.payload))
                    .unwrap_or_default();
                rows.push(json!({
                    "id": el.id,
                    "kind": el.kind,
                    "display_label": el.display_label,
                    "snippet": snippet,
                }));
            }
            Ok(json!({ "content": [text_block(&truncate_json(&rows).map_err(tool_err)?)] }))
        }
        "get_catalog_element" => {
            let element_id = require_str(&args, "element_id").map_err(tool_err)?;
            let detail = store
                .get_element(element_id)
                .map_err(tool_err)?
                .ok_or_else(|| tool_err_msg(format!("unknown element `{element_id}`")))?;
            let body = json_full(&detail).map_err(tool_err)?;
            Ok(json!({ "content": [text_block(&body)] }))
        }
        "get_catalog_revision" => {
            let revision_id = require_str(&args, "revision_id").map_err(tool_err)?;
            let rev = store
                .get_revision(revision_id)
                .map_err(tool_err)?
                .ok_or_else(|| tool_err_msg(format!("unknown revision `{revision_id}`")))?;
            let body = json_full(&rev).map_err(tool_err)?;
            Ok(json!({ "content": [text_block(&body)] }))
        }
        "get_element_usage" => {
            let element_id = require_str(&args, "element_id").map_err(tool_err)?;
            let usage = store.element_usage(element_id).map_err(tool_err)?;
            let body = json_full(&usage).map_err(tool_err)?;
            Ok(json!({ "content": [text_block(&body)] }))
        }
        "get_skill_evidence" => {
            let element_id = require_str(&args, "element_id").map_err(tool_err)?;
            let links = store
                .get_element_skill_links(element_id)
                .map_err(tool_err)?;
            Ok(json!({ "content": [text_block(&truncate_json(&links).map_err(tool_err)?)] }))
        }
        "posting_hints" => {
            let id = require_str(&args, "application_id").map_err(tool_err)?;
            let hints = store.application_posting_hints(id).map_err(tool_err)?;
            Ok(json!({ "content": [text_block(&truncate_json(&hints).map_err(tool_err)?)] }))
        }
        "analyze_application" => {
            let id = require_str(&args, "application_id").map_err(tool_err)?;
            let report = store.application_analyze(id).map_err(tool_err)?;
            Ok(json!({ "content": [text_block(&truncate_json(&report).map_err(tool_err)?)] }))
        }
        "get_composition" => {
            let id = require_str(&args, "application_id").map_err(tool_err)?;
            let app = store
                .get_application(id)
                .map_err(tool_err)?
                .ok_or_else(|| tool_err_msg("application not found"))?;
            Ok(json!({
                "content": [text_block(&truncate_json(&app.draft_composition).map_err(tool_err)?)]
            }))
        }
        other => Err(json!({"code": -32601, "message": format!("unknown tool `{other}`")})),
    }
}

pub fn tool_err(e: impl ToString) -> Value {
    json!({"code": -32000, "message": e.to_string()})
}

fn tool_err_msg(message: impl Into<String>) -> Value {
    json!({"code": -32000, "message": message.into()})
}

fn require_str<'a>(args: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .with_context(|| format!("missing `{key}`"))
}

pub fn text_block(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn truncate_json<T: Serialize>(value: &T) -> anyhow::Result<String> {
    let s = serde_json::to_string_pretty(value)?;
    Ok(truncate_str(&s))
}

fn json_full<T: Serialize>(value: &T) -> anyhow::Result<String> {
    let s = serde_json::to_string_pretty(value)?;
    if s.len() > MAX_CATALOG_JSON_BYTES {
        anyhow::bail!(
            "response size {} bytes exceeds limit {} — fetch a single revision with get_catalog_revision",
            s.len(),
            MAX_CATALOG_JSON_BYTES
        );
    }
    Ok(s)
}

fn truncate_str(s: &str) -> String {
    if s.len() <= MAX_SUMMARY_TEXT_CHARS {
        return s.to_string();
    }
    format!("{}… [truncated]", &s[..MAX_SUMMARY_TEXT_CHARS])
}

fn payload_snippet(payload: &Value) -> String {
    truncate_str(&payload.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use store::CreateElementRequest;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("jobdesk.db");
        (dir, Store::open(&db).unwrap())
    }

    #[test]
    fn get_catalog_element_returns_full_payload() {
        let (_dir, store) = temp_store();
        store
            .create_element(&CreateElementRequest {
                id: "project.mcp-test".into(),
                kind: "project".into(),
                payload: serde_json::json!({
                    "name": "MCP Test",
                    "details": ["alpha", "beta"]
                }),
                parent_element_id: None,
            })
            .unwrap();

        let out = call_tool(
            &store,
            "get_catalog_element",
            json!({"element_id": "project.mcp-test"}),
        )
        .unwrap();
        let text = out["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("MCP Test"));
        assert!(text.contains("alpha"));
        assert!(!text.contains("[truncated]"));
    }

    #[test]
    fn get_catalog_revision_missing_id_errors() {
        let (_dir, store) = temp_store();
        let err = call_tool(
            &store,
            "get_catalog_revision",
            json!({"revision_id": "no-such-rev"}),
        )
        .unwrap_err();
        assert_eq!(err["code"], -32000);
    }

    #[test]
    fn get_catalog_element_unknown_id_errors() {
        let (_dir, store) = temp_store();
        let err = call_tool(
            &store,
            "get_catalog_element",
            json!({"element_id": "missing"}),
        )
        .unwrap_err();
        assert!(err["message"].as_str().unwrap().contains("unknown element"));
    }
}
