#![deny(warnings)]

use crate::error::{McpError, Result};
use crate::params::GetSkillParams;
use crate::repo;
use serde_json::Value;

pub fn execute(args: &Value) -> Result<Value> {
    let params: GetSkillParams = serde_json::from_value(args.clone())
        .map_err(|e| McpError::InvalidToolParameters(e.to_string()))?;
    repo::validate_skill_name(&params.name)?;
    let detail = repo::read(&params.name)?;
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&detail)?}],
    }))
}
