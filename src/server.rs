#![deny(warnings)]

// MCP server orchestration

use crate::error::{McpError, Result};
use crate::tools::ToolRegistry;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP server state
pub struct McpServer {
    tool_registry: Arc<ToolRegistry>,
    initialized: Arc<RwLock<bool>>,
}

impl McpServer {
    /// Create a new MCP server.
    pub fn new() -> Result<Self> {
        Ok(Self {
            tool_registry: Arc::new(ToolRegistry::new()),
            initialized: Arc::new(RwLock::new(false)),
        })
    }

    /// Handle the `initialize` request.
    pub async fn handle_initialize(
        &self,
        protocol_version: &str,
        _client_capabilities: &Value,
    ) -> Result<Value> {
        if protocol_version != "2024-11-05"
            && protocol_version != "2025-06-18"
            && protocol_version != "2025-11-25"
        {
            return Err(McpError::InvalidProtocolVersion(protocol_version.to_string()).into());
        }

        let tools = self.tool_registry.list_tools();
        Ok(serde_json::json!({
            "protocolVersion": protocol_version,
            "serverInfo": {
                "name": "skills-mcp",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "tools": {
                    "listChanged": false,
                },
            },
            "tools": tools,
        }))
    }

    /// Handle the `initialized` notification.
    pub async fn handle_initialized(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        *initialized = true;
        Ok(())
    }

    /// Dispatch a tool call.
    pub async fn handle_tool_call(&self, tool_name: &str, arguments: &Value) -> Result<Value> {
        self.tool_registry.execute_tool(tool_name, arguments).await
    }

    /// Handle the `shutdown` request.
    pub async fn handle_shutdown(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        *initialized = false;
        Ok(())
    }

    /// Return list of tools in MCP schema format.
    pub fn list_tools(&self) -> Value {
        self.tool_registry.list_tools()
    }

    /// Return whether the server has been initialized.
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn initialize_marks_server_initialized() {
        let server = McpServer::new().unwrap();
        assert!(!server.is_initialized().await);
        server
            .handle_initialize("2024-11-05", &json!({}))
            .await
            .unwrap();
        assert!(
            server.is_initialized().await,
            "tools/list right after initialize must work without a separate notification"
        );
    }

    #[tokio::test]
    async fn initialize_accepts_current_claude_code_version() {
        let server = McpServer::new().unwrap();
        server
            .handle_initialize("2025-03-26", &json!({}))
            .await
            .expect("2025-03-26 (current Claude Code) must be accepted");
        let server = McpServer::new().unwrap();
        server
            .handle_initialize("2024-11-05", &json!({}))
            .await
            .expect("2024-11-05 must be accepted");
    }

    #[tokio::test]
    async fn initialize_rejects_unknown_version() {
        let server = McpServer::new().unwrap();
        assert!(server.handle_initialize("1999-01-01", &json!({})).await.is_err());
    }

    #[tokio::test]
    async fn initialize_result_has_no_top_level_tools_key() {
        let server = McpServer::new().unwrap();
        let result = server.handle_initialize("2024-11-05", &json!({})).await.unwrap();
        assert!(
            result.get("tools").is_none(),
            "tools belong to tools/list, not the initialize result"
        );
        assert!(result.get("protocolVersion").is_some());
        assert!(result.get("capabilities").is_some());
    }
}
