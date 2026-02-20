#![deny(warnings)]

// Search skills by query string

use crate::db::{SkillDb, SkillKind};
use crate::error::{McpError, Result};
use serde_json::Value;
use std::str::FromStr;

/// Parse arguments and search skills in the database.
pub fn execute(args: &Value, db: &SkillDb) -> Result<Value> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidToolParameters("Missing required parameter: query".to_string()))?;

    let kind: Option<SkillKind> = args
        .get("kind")
        .and_then(|v| v.as_str())
        .map(SkillKind::from_str)
        .transpose()?;

    let skills = db.search(query, kind.as_ref());
    let text = serde_json::to_string_pretty(&skills)?;
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}]
    }))
}
