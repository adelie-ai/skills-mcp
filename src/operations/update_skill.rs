#![deny(warnings)]

use crate::error::{McpError, Result};
use crate::params::UpdateSkillParams;
use crate::repo::{self, UpdatePatch};
use serde_json::Value;

pub fn execute(args: &Value) -> Result<String> {
    let params: UpdateSkillParams = serde_json::from_value(args.clone())
        .map_err(|e| McpError::InvalidToolParameters(e.to_string()))?;
    repo::validate_skill_name(&params.name)?;
    if let Some(new_name) = &params.new_name {
        repo::validate_skill_name(new_name)?;
    }
    let detail = repo::write_update(
        &params.name,
        UpdatePatch {
            name: params.new_name,
            description: params.description,
            content: params.content,
            tags: params.tags,
        },
    )?;
    Ok(serde_json::to_string_pretty(&detail)?)
}
