#![allow(deprecated)] // call_tool is deprecated in favor of request_tool_call

use std::sync::Arc;

use assert_cmd::cargo::cargo_bin;
use async_trait::async_trait;
use rust_mcp_sdk::mcp_client::{ClientHandler, ClientRuntime, McpClientOptions, client_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams,
};
use rust_mcp_sdk::{McpClient, StdioTransport, ToMcpClientHandler, TransportOptions};
use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn kb_bin() -> String {
    cargo_bin("kb").to_str().unwrap().to_string()
}

fn init_project_with_domain(domain: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    std::process::Command::new(kb_bin())
        .args(["init"])
        .current_dir(dir.path())
        .status()
        .expect("kb init failed");
    std::process::Command::new(kb_bin())
        .args(["add", domain])
        .current_dir(dir.path())
        .status()
        .expect("kb add failed");
    dir
}

struct TestHandler;

#[async_trait]
impl ClientHandler for TestHandler {}

async fn create_client(cwd: &std::path::Path) -> Arc<ClientRuntime> {
    let details = InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "kb-test".into(),
            version: "0.1.0".into(),
            title: None,
            description: None,
            icons: vec![],
            website_url: None,
        },
        protocol_version: "2025-11-25".into(),
        meta: None,
    };

    // StdioTransport doesn't support setting cwd, so we use a shell wrapper
    let cwd_str = cwd.to_str().unwrap().replace('\\', "/");
    let kb = kb_bin().replace('\\', "/");
    let transport = StdioTransport::create_with_server_launch(
        "bash",
        vec!["-c".to_string(), format!("cd '{cwd_str}' && '{kb}' mcp")],
        None,
        TransportOptions {
            timeout: Duration::from_secs(15),
        },
    )
    .expect("transport creation failed");

    let handler = TestHandler;
    let client = client_runtime::create_client(McpClientOptions {
        client_details: details,
        transport,
        handler: handler.to_mcp_client_handler(),
        task_store: None,
        server_task_store: None,
    });
    client.clone().start().await.expect("client start failed");
    client
}

fn tool_call(name: &str, args: serde_json::Value) -> CallToolRequestParams {
    CallToolRequestParams {
        name: name.to_string(),
        arguments: args.as_object().cloned(),
        meta: None,
        task: None,
    }
}

fn result_text(r: &rust_mcp_sdk::schema::CallToolResult) -> String {
    r.content
        .first()
        .and_then(|c| c.as_text_content().ok())
        .map(|t| t.text.clone())
        .unwrap_or_default()
}

fn result_json(r: &rust_mcp_sdk::schema::CallToolResult) -> serde_json::Value {
    serde_json::from_str(&result_text(r)).unwrap_or(json!(null))
}

/// Extract record ID from query result, handling both flat and tagged enum formats
fn extract_record_id(data: &serde_json::Value, idx: usize) -> String {
    let record = &data["records"][idx];
    // Flat format: {"id": "..."}
    record["id"]
        .as_str()
        // Tagged enum format: {"Convention": {"id": "..."}}
        .or_else(|| {
            record
                .as_object()
                .and_then(|o| o.values().next())
                .and_then(|v| v["id"].as_str())
        })
        .unwrap_or("unknown")
        .to_string()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tools_list_returns_13_tools() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;

    let tools = client.list_tools(None).await.unwrap().tools;
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    assert_eq!(tools.len(), 13, "got: {names:?}");
    for expected in [
        "kb_prime",
        "kb_session_resume",
        "kb_session_end",
        "kb_query",
        "kb_query_all",
        "kb_search",
        "kb_record",
        "kb_edit",
        "kb_delete",
        "kb_status",
        "kb_oracle",
        "kb_feedback",
        "kb_check",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
}

#[tokio::test]
async fn prime_returns_session_id() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;

    let r = client
        .call_tool(tool_call("kb_prime", json!({"label": "int-test"})))
        .await
        .unwrap();
    let data = result_json(&r);

    assert!(data["session_id"].is_string(), "no session_id: {data}");
    assert_eq!(data["label"], "int-test");
}

#[tokio::test]
async fn record_query_search_delete_lifecycle() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;
    client
        .call_tool(tool_call("kb_prime", json!({})))
        .await
        .unwrap();

    // Record
    let r = client
        .call_tool(tool_call(
            "kb_record",
            json!({
                "domain": "test", "record_type": "convention",
                "description": "Always use snake_case for functions"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(result_json(&r)["success"], true);

    // Query
    let r = client
        .call_tool(tool_call("kb_query", json!({"domain": "test"})))
        .await
        .unwrap();
    let data = result_json(&r);
    assert_eq!(data["count"], 1);

    // Search
    let r = client
        .call_tool(tool_call("kb_search", json!({"query": "snake_case"})))
        .await
        .unwrap();
    assert_eq!(result_json(&r)["total"], 1);

    // Delete
    let id = extract_record_id(&data, 0);
    let r = client
        .call_tool(tool_call(
            "kb_delete",
            json!({"domain": "test", "entry_id": id}),
        ))
        .await
        .unwrap();
    assert_eq!(result_json(&r)["success"], true);

    // Verify empty
    let r = client
        .call_tool(tool_call("kb_query", json!({"domain": "test"})))
        .await
        .unwrap();
    assert_eq!(result_json(&r)["count"], 0);
}

#[tokio::test]
async fn record_and_edit() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;
    client
        .call_tool(tool_call("kb_prime", json!({})))
        .await
        .unwrap();

    client
        .call_tool(tool_call(
            "kb_record",
            json!({
                "domain": "test", "record_type": "pattern",
                "name": "error-handling", "description": "Use Result<T, E>"
            }),
        ))
        .await
        .unwrap();

    let r = client
        .call_tool(tool_call("kb_query", json!({"domain": "test"})))
        .await
        .unwrap();
    let data = result_json(&r);
    let id = extract_record_id(&data, 0);

    let r = client
        .call_tool(tool_call(
            "kb_edit",
            json!({
                "domain": "test", "entry_id": id,
                "updates": {"description": "Use anyhow::Result"}
            }),
        ))
        .await
        .unwrap();
    assert_eq!(result_json(&r)["success"], true);
}

#[tokio::test]
async fn query_all_and_status() {
    let dir = init_project_with_domain("test");
    std::process::Command::new(kb_bin())
        .args(["add", "infra"])
        .current_dir(dir.path())
        .status()
        .unwrap();

    let client = create_client(dir.path()).await;
    client
        .call_tool(tool_call("kb_prime", json!({})))
        .await
        .unwrap();

    client
        .call_tool(tool_call(
            "kb_record",
            json!({
                "domain": "test", "record_type": "convention", "description": "c1"
            }),
        ))
        .await
        .unwrap();
    client
        .call_tool(tool_call(
            "kb_record",
            json!({
                "domain": "infra", "record_type": "convention", "description": "c2"
            }),
        ))
        .await
        .unwrap();

    let r = client
        .call_tool(tool_call("kb_query_all", json!({})))
        .await
        .unwrap();
    let domains = result_json(&r)["domains"].as_array().unwrap().len();
    assert_eq!(domains, 2);

    let r = client
        .call_tool(tool_call("kb_status", json!({})))
        .await
        .unwrap();
    let domains = result_json(&r)["domains"].as_array().unwrap().len();
    assert_eq!(domains, 2);
}

#[tokio::test]
async fn oracle_records_gap() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;
    client
        .call_tool(tool_call("kb_prime", json!({})))
        .await
        .unwrap();

    let r = client
        .call_tool(tool_call(
            "kb_oracle",
            json!({
                "query": "How do we handle auth?", "domain": "test"
            }),
        ))
        .await
        .unwrap();
    let text = result_text(&r).to_lowercase();
    assert!(
        text.contains("gap") || text.contains("recorded"),
        "oracle: {text}"
    );
}

#[tokio::test]
async fn feedback_on_entry() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;
    client
        .call_tool(tool_call("kb_prime", json!({})))
        .await
        .unwrap();

    client
        .call_tool(tool_call(
            "kb_record",
            json!({
                "domain": "test", "record_type": "convention", "description": "fb target"
            }),
        ))
        .await
        .unwrap();

    let r = client
        .call_tool(tool_call("kb_query", json!({"domain": "test"})))
        .await
        .unwrap();
    let id = extract_record_id(&result_json(&r), 0);

    let r = client
        .call_tool(tool_call(
            "kb_feedback",
            json!({
                "entry_id": id, "signal": "helpful", "domain": "test"
            }),
        ))
        .await
        .unwrap();
    let text = result_text(&r);
    assert!(
        text.contains("Feedback") || text.contains("recorded"),
        "{text}"
    );
}

#[tokio::test]
async fn check_references() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;
    client
        .call_tool(tool_call("kb_prime", json!({})))
        .await
        .unwrap();

    let r = client
        .call_tool(tool_call("kb_check", json!({"domain": "test"})))
        .await
        .unwrap();
    let data = result_json(&r);
    assert!(data["broken_count"].is_number(), "check: {data}");
}

#[tokio::test]
async fn session_end_and_resume() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;

    let r = client
        .call_tool(tool_call("kb_prime", json!({"label": "resumable"})))
        .await
        .unwrap();
    let sid = result_json(&r)["session_id"].as_str().unwrap().to_string();

    let r = client
        .call_tool(tool_call("kb_session_end", json!({})))
        .await
        .unwrap();
    assert!(result_text(&r).contains("ended"));

    // Resume with new client
    let client2 = create_client(dir.path()).await;
    let r = client2
        .call_tool(tool_call("kb_session_resume", json!({"session_id": sid})))
        .await
        .unwrap();
    let text = result_text(&r);
    // Verify the session ID appears somewhere in the response
    assert!(
        text.contains(&sid),
        "resume response should reference session: {text}"
    );
}

#[tokio::test]
async fn session_end_without_prime_errors() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;

    // MCP tools return errors as isError=true in the result, not as Err()
    let r = client
        .call_tool(tool_call("kb_session_end", json!({})))
        .await;
    match r {
        Err(_) => {} // expected
        Ok(result) => {
            // Some MCP implementations return Ok with isError flag
            assert!(
                result.is_error.unwrap_or(false)
                    || result_text(&result).to_lowercase().contains("error")
                    || result_text(&result)
                        .to_lowercase()
                        .contains("no active session"),
                "session_end without prime should indicate error"
            );
        }
    }
}

#[tokio::test]
async fn access_log_written_after_operations() {
    let dir = init_project_with_domain("test");
    let client = create_client(dir.path()).await;

    client
        .call_tool(tool_call("kb_prime", json!({})))
        .await
        .unwrap();
    client
        .call_tool(tool_call(
            "kb_record",
            json!({
                "domain": "test", "record_type": "convention", "description": "log test"
            }),
        ))
        .await
        .unwrap();
    client
        .call_tool(tool_call("kb_query", json!({"domain": "test"})))
        .await
        .unwrap();
    client
        .call_tool(tool_call("kb_session_end", json!({})))
        .await
        .unwrap();

    let log = dir.path().join(".kb/access.jsonl");
    assert!(log.exists(), "access.jsonl should exist");
    let content = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        lines.len() >= 4,
        "expected >=4 log entries, got {}",
        lines.len()
    );
}
