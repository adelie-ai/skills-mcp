#![deny(warnings)]

// Delete a skill by ID or name

use crate::db::SkillDb;
use crate::error::{McpError, Result};
use serde_json::Value;

/// Parse arguments and delete a skill from the database.
pub fn execute(args: &Value, db: &mut SkillDb) -> Result<Value> {
    let id_or_name = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidToolParameters("Missing required parameter: id".to_string()))?;

    let deleted = db.delete(id_or_name)?;
    let text = format!(
        "Deleted skill '{}' (id: {})",
        deleted.name, deleted.id
    );
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}]
    }))
}
