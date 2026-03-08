use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, RpcError,
    schema_utils::CallToolError,
};
use tokio::sync::Mutex;

use super::tools::*;

pub struct KbServerHandler {
    pub cwd: PathBuf,
    pub current_session_id: Arc<Mutex<Option<String>>>,
}

impl KbServerHandler {
    async fn session_id(&self) -> Option<String> {
        self.current_session_id.lock().await.clone()
    }
}

#[async_trait]
impl ServerHandler for KbServerHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: KbTools::tools(),
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        eprintln!("[kb-mcp] calling tool: {}", params.name);

        let tool: KbTools = KbTools::try_from(params).map_err(CallToolError::new)?;
        let sid = self.session_id().await;
        let sid_ref = sid.as_deref();

        match tool {
            KbTools::KbPrimeTool(t) => {
                let (result, new_sid) = t.call_tool(&self.cwd)?;
                *self.current_session_id.lock().await = Some(new_sid);
                Ok(result)
            }
            KbTools::KbSessionResumeTool(t) => {
                let (result, new_sid) = t.call_tool(&self.cwd)?;
                *self.current_session_id.lock().await = Some(new_sid);
                Ok(result)
            }
            KbTools::KbSessionEndTool(t) => {
                let session_id = sid
                    .ok_or_else(|| CallToolError::from_message("No active session".to_string()))?;
                let result = t.call_tool(&self.cwd, &session_id)?;
                *self.current_session_id.lock().await = None;
                Ok(result)
            }
            KbTools::KbQueryTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbQueryAllTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbSearchTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbRecordTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbEditTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbDeleteTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbStatusTool(t) => t.call_tool(&self.cwd),
            KbTools::KbOracleTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbFeedbackTool(t) => t.call_tool(&self.cwd, sid_ref),
            KbTools::KbCheckTool(t) => t.call_tool(&self.cwd, sid_ref),
        }
    }
}
