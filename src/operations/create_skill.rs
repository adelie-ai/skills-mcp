#![deny(warnings)]

// Create a new skill (code snippet or how-to document)

use crate::db::{CreateSkillRequest, SkillDb, SkillKind};
use crate::error::{McpError, Result};
use serde_json::Value;
use std::str::FromStr;

/// Parse arguments and create a new skill in the database.
pub fn execute(args: &Value, db: &mut SkillDb) -> Result<Value> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidToolParameters("Missing required parameter: name".to_string()))?
        .to_string();

    let kind_str = args
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidToolParameters("Missing required parameter: kind".to_string()))?;
    let kind = SkillKind::from_str(kind_str)?;

    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidToolParameters("Missing required parameter: content".to_string()))?
        .to_string();

    let language = args
        .get("language")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tags: Option<Vec<String>> = args.get("tags").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
    });

    let skill = db.create(CreateSkillRequest {
        name,
        kind,
        language,
        description,
        content,
        tags,
    })?;

    let text = serde_json::to_string_pretty(&skill)?;
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}]
    }))
}
