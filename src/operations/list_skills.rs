#![deny(warnings)]

// List skills with optional filters

use crate::db::{SkillDb, SkillKind};
use crate::error::Result;
use serde_json::Value;
use std::str::FromStr;

/// Parse arguments and list skills from the database.
pub fn execute(args: &Value, db: &SkillDb) -> Result<Value> {
    let kind: Option<SkillKind> = args
        .get("kind")
        .and_then(|v| v.as_str())
        .map(SkillKind::from_str)
        .transpose()?;

    let tags: Vec<String> = args
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let skills = db.list(kind.as_ref(), &tags);
    let text = serde_json::to_string_pretty(&skills)?;
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}]
    }))
}
