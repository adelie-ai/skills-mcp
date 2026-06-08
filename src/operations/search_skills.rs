#![deny(warnings)]

use crate::error::{McpError, Result};
use crate::params::SearchSkillsParams;
use crate::repo;
use serde_json::Value;

pub fn execute(args: &Value) -> Result<Value> {
    let params: SearchSkillsParams = serde_json::from_value(args.clone())
        .map_err(|e| McpError::InvalidToolParameters(e.to_string()))?;
    let tags = params.tags.unwrap_or_default();
    let results = repo::search(&params.query, &tags);
    let views: Vec<Value> = results
        .iter()
        .map(|s| s.to_view(params.include_paths))
        .collect();
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&views)?}],
    }))
}
