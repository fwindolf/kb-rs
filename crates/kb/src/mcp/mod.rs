mod handler;
#[allow(clippy::enum_variant_names)]
pub mod tools;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rust_mcp_sdk::mcp_server::{McpServerOptions, ServerRuntime, server_runtime};
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::{McpServer, StdioTransport, ToMcpServerHandler, TransportOptions};
use tokio::sync::Mutex;

use handler::KbServerHandler;

pub async fn run_mcp_server(cwd: PathBuf) -> Result<()> {
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "kb-mcp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("KB Knowledge Base MCP Server".to_string()),
            description: Some("MCP server for the kb structured knowledge base".to_string()),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some(
            "Knowledge base server for AI agents. Use kb_prime to start a session.".to_string(),
        ),
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    };

    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| anyhow::anyhow!("Transport error: {e}"))?;

    let handler = KbServerHandler {
        cwd,
        current_session_id: Arc::new(Mutex::new(None)),
    };

    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    eprintln!("[kb-mcp] server starting on stdio");
    if let Err(e) = server.start().await {
        eprintln!("[kb-mcp] server error: {e}");
        return Err(anyhow::anyhow!("MCP server error: {e}"));
    }

    Ok(())
}
