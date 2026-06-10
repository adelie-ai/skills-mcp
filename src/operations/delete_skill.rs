#![deny(warnings)]

use crate::error::{McpError, Result};
use crate::params::DeleteSkillParams;
use crate::repo;
use serde_json::Value;

pub fn execute(args: &Value) -> Result<String> {
    let params: DeleteSkillParams = serde_json::from_value(args.clone())
        .map_err(|e| McpError::InvalidToolParameters(e.to_string()))?;
    repo::validate_skill_name(&params.name)?;
    let deleted = repo::delete(&params.name)?;
    Ok(format!(
        "Deleted skill '{}' from {}",
        deleted.name, deleted.root
    ))
}
