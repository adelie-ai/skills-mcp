#![deny(warnings)]

// Update an existing skill

use crate::db::{SkillDb, SkillKind, UpdateSkillRequest};
use crate::error::{McpError, Result};
use serde_json::Value;
use std::str::FromStr;

/// Parse arguments and update a skill in the database.
pub fn execute(args: &Value, db: &mut SkillDb) -> Result<Value> {
    let id_or_name = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidToolParameters("Missing required parameter: id".to_string()))?;

    let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());

    let kind = args
        .get("kind")
        .and_then(|v| v.as_str())
        .map(SkillKind::from_str)
        .transpose()?;

    // language: present-and-null clears, present-and-string sets, absent leaves unchanged
    let language: Option<Option<String>> = match args.get("language") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => Some(v.as_str().map(|s| s.to_string())),
    };

    let description: Option<Option<String>> = match args.get("description") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(v) => Some(v.as_str().map(|s| s.to_string())),
    };

    let content = args.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());

    let tags: Option<Vec<String>> = args.get("tags").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
    });

    let req = UpdateSkillRequest { name, kind, language, description, content, tags };
    let skill = db.update(id_or_name, req)?;
    let text = serde_json::to_string_pretty(&skill)?;
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}]
    }))
}
