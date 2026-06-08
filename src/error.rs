#![deny(warnings)]

// Error types for skills-mcp (domain errors only; protocol/transport errors
// are handled by mcp-core)

use thiserror::Error;

/// Top-level error type for skills-mcp.
#[derive(Error, Debug)]
pub enum SkillsMcpError {
    /// Skills domain operation errors.
    #[error("Skills error: {0}")]
    Skills(#[from] SkillsError),

    /// JSON serialization/deserialization errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Tool dispatch errors (unknown tool name, bad parameters).
    #[error("MCP tool error: {0}")]
    Mcp(#[from] McpError),

    /// IO errors.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Skills domain errors.
#[derive(Error, Debug)]
pub enum SkillsError {
    /// Skill not found.
    #[error("Skill not found: {0}")]
    NotFound(String),

    /// A skill with this name already exists.
    #[error("Skill already exists: {0}")]
    AlreadyExists(String),

    /// Invalid input data.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Storage read/write failure.
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Tool-dispatch errors (unknown tool, bad parameters).
#[derive(Error, Debug)]
pub enum McpError {
    /// Requested tool does not exist.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool was called with bad parameters.
    #[error("Invalid tool parameters: {0}")]
    InvalidToolParameters(String),
}

/// Convenience `Result` alias.
pub type Result<T> = std::result::Result<T, SkillsMcpError>;
