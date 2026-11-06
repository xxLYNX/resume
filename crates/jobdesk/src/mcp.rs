//! Minimal MCP (Model Context Protocol) server over stdio for local jobdesk workspace tools.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use store::Store;

use crate::mcp_tools;

const PROTOCOL_VERSION: &str = "2024-11-05";

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
            "tools/list" => Ok(json!({ "tools": mcp_tools::tool_definitions() })),
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

fn handle_tool_call(store: &Arc<Mutex<Store>>, params: Value) -> Result<Value, Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| json!({"code": -32602, "message": "missing tool name"}))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let store = store
        .lock()
        .map_err(|_| json!({"code": -32603, "message": "store lock poisoned"}))?;

    mcp_tools::call_tool(&store, name, args)
}
