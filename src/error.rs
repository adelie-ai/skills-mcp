#![deny(warnings)]

// Error types for skills-mcp

use thiserror::Error;

/// Top-level error type for skills-mcp
#[derive(Error, Debug)]
pub enum SkillsMcpError {
    /// Skills operation errors
    #[error("Skills error: {0}")]
    Skills(#[from] SkillsError),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// MCP protocol errors
    #[error("MCP protocol error: {0}")]
    Mcp(#[from] McpError),

    /// Transport layer errors
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Skills domain errors
#[derive(Error, Debug)]
pub enum SkillsError {
    /// Skill not found
    #[error("Skill not found: {0}")]
    NotFound(String),

    /// A skill with this name already exists
    #[error("Skill already exists: {0}")]
    AlreadyExists(String),

    /// Invalid input data
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Storage read/write failure
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// MCP protocol errors
#[derive(Error, Debug)]
pub enum McpError {
    /// Unsupported protocol version
    #[error("Unsupported protocol version: {0}")]
    InvalidProtocolVersion(String),

    /// Malformed JSON-RPC message
    #[error("Invalid JSON-RPC message: {0}")]
    InvalidJsonRpc(String),

    /// Requested tool does not exist
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool was called with bad parameters
    #[error("Invalid tool parameters: {0}")]
    InvalidToolParameters(String),
}

/// Transport layer errors
#[derive(Error, Debug)]
pub enum TransportError {
    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Malformed message
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    /// Connection was closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Underlying IO error
    #[error("Transport IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience Result alias
pub type Result<T> = std::result::Result<T, SkillsMcpError>;
