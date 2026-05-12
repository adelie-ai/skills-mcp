#![deny(warnings)]

use crate::error::{McpError, Result};
use crate::params::ListSkillsParams;
use crate::repo;
use serde_json::Value;

pub fn execute(args: &Value) -> Result<Value> {
    let params: ListSkillsParams = serde_json::from_value(args.clone())
        .map_err(|e| McpError::InvalidToolParameters(e.to_string()))?;
    let required = params.tags.unwrap_or_default();
    let mut skills = repo::list_all();
    if !required.is_empty() {
        skills.retain(|s| required.iter().any(|t| s.tags.iter().any(|x| x == t)));
    }
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&skills)?}],
    }))
}
