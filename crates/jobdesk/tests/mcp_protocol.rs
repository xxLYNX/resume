//! End-to-end MCP stdio JSON-RPC against the `jobdesk` binary.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};
use store::{import_legacy_content, CreateElementRequest, ImportOptions, Store};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rpc_line(method: &str, id: i64, params: Value) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    }))
    .unwrap()
}

fn read_response(reader: &mut impl BufRead, id: i64) -> Value {
    for line in reader.lines() {
        let line = line.expect("mcp stdout");
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line).expect("json line");
        if v.get("id") == Some(&json!(id)) {
            return v;
        }
    }
    panic!("no response for id {id}");
}

fn call_tool_stdio(
    _workspace: &Path,
    id: i64,
    name: &str,
    arguments: Value,
    reader: &mut impl BufRead,
    stdin: &mut impl Write,
) -> Value {
    writeln!(
        stdin,
        "{}",
        rpc_line(
            "tools/call",
            id,
            json!({ "name": name, "arguments": arguments }),
        )
    )
    .unwrap();
    stdin.flush().unwrap();
    read_response(reader, id)
}

#[test]
fn mcp_stdio_initialize_discover_and_catalog_tools() {
    let ws = tempfile::tempdir().unwrap();
    let ws_path = ws.path();
    let store = Store::open(&ws_path.join("jobdesk.db")).unwrap();
    import_legacy_content(&repo_root(), &store, ImportOptions::default()).unwrap();

    let canonical = store
        .get_element("project.cybersecurity-lab")
        .unwrap()
        .expect("cybersecurity lab");
    let rev_id = canonical.latest.as_ref().unwrap().id.clone();

    let bin = env!("CARGO_BIN_EXE_jobdesk");
    let mut child = Command::new(bin)
        .args([
            "--root",
            &repo_root().to_string_lossy(),
            "--workspace",
            &ws_path.to_string_lossy(),
            "mcp",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jobdesk mcp");

    let mut stdin = child.stdin.take().expect("stdin");
    let mut reader = BufReader::new(child.stdout.take().expect("stdout"));

    writeln!(
        stdin,
        "{}",
        rpc_line(
            "initialize",
            1,
            json!({ "protocolVersion": "2024-11-05", "capabilities": {} }),
        )
    )
    .unwrap();
    stdin.flush().unwrap();
    assert!(read_response(&mut reader, 1).get("result").is_some());

    writeln!(stdin, "{}", rpc_line("tools/list", 2, json!({}))).unwrap();
    stdin.flush().unwrap();
    let tools = read_response(&mut reader, 2);
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected in [
        "get_catalog_element",
        "get_catalog_revision",
        "get_element_usage",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }

    let elem = call_tool_stdio(
        ws_path,
        3,
        "get_catalog_element",
        json!({ "element_id": "project.cybersecurity-lab" }),
        &mut reader,
        &mut stdin,
    );
    assert!(elem.get("result").is_some(), "{elem}");
    let text = elem["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["id"], "project.cybersecurity-lab");
    assert_eq!(
        parsed["latest"]["payload"],
        canonical.latest.as_ref().unwrap().payload
    );

    let rev = call_tool_stdio(
        ws_path,
        4,
        "get_catalog_revision",
        json!({ "revision_id": rev_id }),
        &mut reader,
        &mut stdin,
    );
    let rev_text = rev["result"]["content"][0]["text"].as_str().unwrap();
    let rev_parsed: Value = serde_json::from_str(rev_text).unwrap();
    assert_eq!(rev_parsed["id"], rev_id);

    let usage = call_tool_stdio(
        ws_path,
        5,
        "get_element_usage",
        json!({ "element_id": "project.cybersecurity-lab" }),
        &mut reader,
        &mut stdin,
    );
    let usage_text = usage["result"]["content"][0]["text"].as_str().unwrap();
    let usage_parsed: Value = serde_json::from_str(usage_text).unwrap();
    assert!(usage_parsed["applications"].is_array());

    let missing = call_tool_stdio(
        ws_path,
        6,
        "get_catalog_element",
        json!({ "element_id": "does.not.exist" }),
        &mut reader,
        &mut stdin,
    );
    assert!(missing.get("error").is_some(), "expected json-rpc error");
    assert!(missing["error"]["message"]
        .as_str()
        .unwrap()
        .contains("unknown element"));
}

#[test]
fn mcp_stdio_oversized_element_returns_error_not_truncation() {
    let ws = tempfile::tempdir().unwrap();
    let ws_path = ws.path();
    let store = Store::open(&ws_path.join("jobdesk.db")).unwrap();
    let big = "x".repeat(600_000);
    store
        .create_element(&CreateElementRequest {
            id: "project.oversized".into(),
            kind: "project".into(),
            payload: serde_json::json!({ "name": "Big", "details": [big] }),
            parent_element_id: None,
        })
        .unwrap();

    let bin = env!("CARGO_BIN_EXE_jobdesk");
    let mut child = Command::new(bin)
        .args([
            "--root",
            &repo_root().to_string_lossy(),
            "--workspace",
            &ws_path.to_string_lossy(),
            "mcp",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    writeln!(
        stdin,
        "{}",
        rpc_line(
            "initialize",
            1,
            json!({ "protocolVersion": "2024-11-05", "capabilities": {} }),
        )
    )
    .unwrap();
    stdin.flush().unwrap();
    read_response(&mut reader, 1);

    let resp = call_tool_stdio(
        ws_path,
        2,
        "get_catalog_element",
        json!({ "element_id": "project.oversized" }),
        &mut reader,
        &mut stdin,
    );
    assert!(resp.get("error").is_some());
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("exceeds limit"));
    assert!(!msg.contains("[truncated]"));
}
