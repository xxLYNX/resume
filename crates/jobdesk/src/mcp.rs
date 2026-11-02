//! Minimal MCP (Model Context Protocol) server over stdio for local jobdesk workspace tools.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use store::{ListApplicationsQuery, Store};

const PROTOCOL_VERSION: &str = "2024-11-05";
const MAX_TEXT_CHARS: usize = 12_000;

pub fn run_mcp(workspace_db: PathBuf) -> Result<()> {
    let store = Arc::new(Mutex::new(
        Store::open(&workspace_db).context("open workspace database")?,
    ));
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                write_response(
                    &mut stdout,
                    None,
                    json!({"code": -32700, "message": format!("parse error: {e}")}),
                )?;
                continue;
            }
        };
        let id = req.id.clone();
        let result = match req.method.as_str() {
            "initialize" => Ok(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "jobdesk", "version": env!("CARGO_PKG_VERSION") }
            })),
            "notifications/initialized" | "initialized" => continue,
            "tools/list" => Ok(json!({ "tools": tool_definitions() })),
            "tools/call" => handle_tool_call(&store, req.params),
            "ping" => Ok(json!({})),
            other => Err(json!({"code": -32601, "message": format!("unknown method `{other}`")})),
        };
        match result {
            Ok(value) => write_response(&mut stdout, id, value)?,
            Err(err) => write_response(&mut stdout, id, err)?,
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

fn write_response(out: &mut impl Write, id: Option<Value>, result: Value) -> Result<()> {
    let body = if result.get("code").is_some() && result.get("message").is_some() {
        json!({ "jsonrpc": "2.0", "id": id, "error": result })
    } else {
        json!({ "jsonrpc": "2.0", "id": id, "result": result })
    };
    writeln!(out, "{}", body)?;
    out.flush()?;
    Ok(())
}

fn tool_definitions() -> Value {
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

fn handle_tool_call(store: &Arc<Mutex<Store>>, params: Value) -> Result<Value, Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| json!({"code": -32602, "message": "missing tool name"}))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or(json!({}));

    let store = store.lock().map_err(|_| {
        json!({"code": -32603, "message": "store lock poisoned"})
    })?;

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
                .ok_or_else(|| json!({"code": -32000, "message": "application not found"}))?;
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
        "get_skill_evidence" => {
            let element_id = require_str(&args, "element_id").map_err(tool_err)?;
            let links = store.get_element_skill_links(element_id).map_err(tool_err)?;
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
                .ok_or_else(|| json!({"code": -32000, "message": "application not found"}))?;
            Ok(json!({
                "content": [text_block(&truncate_json(&app.draft_composition).map_err(tool_err)?)]
            }))
        }
        other => Err(json!({"code": -32601, "message": format!("unknown tool `{other}`")})),
    }
}

fn tool_err(e: impl ToString) -> Value {
    json!({"code": -32000, "message": e.to_string()})
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .with_context(|| format!("missing `{key}`"))
}

fn text_block(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn truncate_json<T: Serialize>(value: &T) -> Result<String> {
    let s = serde_json::to_string_pretty(value)?;
    Ok(truncate_str(&s))
}

fn truncate_str(s: &str) -> String {
    if s.len() <= MAX_TEXT_CHARS {
        return s.to_string();
    }
    format!("{}… [truncated]", &s[..MAX_TEXT_CHARS])
}

fn payload_snippet(payload: &Value) -> String {
    let s = payload.to_string();
    truncate_str(&s)
}
