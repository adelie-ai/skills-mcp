#![deny(warnings)]

// Create a new skill directory + SKILL.md on disk.

use crate::error::{McpError, Result, SkillsError};
use crate::params::CreateSkillParams;
use crate::repo::{self, SkillFrontmatter};
use serde_json::Value;

pub fn execute(args: &Value) -> Result<Value> {
    let params: CreateSkillParams = serde_json::from_value(args.clone())
        .map_err(|e| McpError::InvalidToolParameters(e.to_string()))?;
    if params.content.trim().is_empty() {
        return Err(SkillsError::InvalidInput("content must not be empty".into()).into());
    }
    let fm = SkillFrontmatter {
        name: params.name.clone(),
        description: params.description,
        tags: params.tags.unwrap_or_default(),
    };
    let detail = repo::write_new(&params.name, &fm, &params.content)?;
    text_response(serde_json::to_string_pretty(&detail)?)
}

fn text_response(text: String) -> Result<Value> {
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}],
    }))
}
