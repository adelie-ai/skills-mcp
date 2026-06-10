#![deny(warnings)]

use crate::error::{McpError, Result};
use crate::params::ListSkillsParams;
use crate::repo;
use serde_json::Value;

pub fn execute(args: &Value) -> Result<String> {
    let params: ListSkillsParams = serde_json::from_value(args.clone())
        .map_err(|e| McpError::InvalidToolParameters(e.to_string()))?;
    let required = params.tags.unwrap_or_default();
    let mut skills = repo::list_all();
    if !required.is_empty() {
        skills.retain(|s| required.iter().any(|t| s.tags.iter().any(|x| x == t)));
    }
    let views: Vec<Value> = skills
        .iter()
        .map(|s| s.to_view(params.include_paths))
        .collect();
    Ok(serde_json::to_string_pretty(&views)?)
}
