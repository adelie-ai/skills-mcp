#![deny(warnings)]

// Retrieve a skill by ID or name

use crate::db::SkillDb;
use crate::error::{McpError, Result};
use serde_json::Value;

/// Parse arguments and retrieve a skill from the database.
pub fn execute(args: &Value, db: &SkillDb) -> Result<Value> {
    let id_or_name = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidToolParameters("Missing required parameter: id".to_string()))?;

    let skill = db.get(id_or_name)?;
    let text = serde_json::to_string_pretty(&skill)?;
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}]
    }))
}
