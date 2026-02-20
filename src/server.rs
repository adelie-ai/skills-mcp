#![deny(warnings)]

// MCP server orchestration

use crate::error::{McpError, Result};
use crate::tools::ToolRegistry;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP server state
pub struct McpServer {
    tool_registry: Arc<ToolRegistry>,
    initialized: Arc<RwLock<bool>>,
}

impl McpServer {
    /// Create a new MCP server backed by the database at `db_path`.
    pub fn new(db_path: &Path) -> Result<Self> {
        Ok(Self {
            tool_registry: Arc::new(ToolRegistry::new(db_path)?),
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
