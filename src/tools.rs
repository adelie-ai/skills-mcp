#![deny(warnings)]

// Tool registry: MCP tool definitions and dispatch.
//
// Backing storage is the on-disk Anthropic Agent Skills format (one
// SKILL.md per skill directory). See `crate::repo` for the layout. There
// is no shared in-memory state — every operation reads from / writes to
// disk directly — so the registry is stateless.

use crate::error::{McpError, Result};
use crate::params::{
    CreateSkillParams, DeleteSkillParams, GetSkillParams, ListSkillsParams, SearchSkillsParams,
    UpdateSkillParams,
};
use schemars::{JsonSchema, schema_for};
use serde_json::Value;

#[derive(Default)]
pub struct ToolRegistry;

impl ToolRegistry {
    pub fn new() -> Self {
        Self
    }

    /// Return all tool definitions in MCP JSON schema format.
    pub fn list_tools(&self) -> Value {
        serde_json::json!([
            tool_def::<CreateSkillParams>(
                "skills_create_skill",
                "Create a new skill on disk as <root>/<name>/SKILL.md with YAML frontmatter (name, description, optional tags) plus a markdown body. Writes to ~/.agents/skills, creating it if missing."
            ),
            tool_def::<GetSkillParams>(
                "skills_get_skill",
                "Read a skill by name. Searches all configured roots (~/.agents/skills, ~/.claude/skills, and any in $SKILLS_MCP_ROOTS) and returns the parsed frontmatter, body, absolute path, and a list of attachment filenames in the skill directory."
            ),
            tool_def::<UpdateSkillParams>(
                "skills_update_skill",
                "Modify an existing skill in place. Only the fields you set are changed. If `new_name` is provided the skill directory is renamed."
            ),
            tool_def::<DeleteSkillParams>(
                "skills_delete_skill",
                "Permanently delete a skill directory (SKILL.md and any attachments) by name."
            ),
            tool_def::<ListSkillsParams>(
                "skills_list_skills",
                "List every skill across all configured roots. Returns name, description, tags, path, root, and attachment filenames for each. Optionally filter by tag."
            ),
            tool_def::<SearchSkillsParams>(
                "skills_search_skills",
                "Case-insensitive full-text search across skill names, descriptions, tags, and SKILL.md bodies. Optionally restrict to skills carrying at least one of the supplied tags."
            ),
        ])
    }

    /// Dispatch a tool call to the appropriate operation.
    pub async fn execute_tool(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "skills_create_skill" => crate::operations::create_skill::execute(args),
            "skills_get_skill" => crate::operations::get_skill::execute(args),
            "skills_update_skill" => crate::operations::update_skill::execute(args),
            "skills_delete_skill" => crate::operations::delete_skill::execute(args),
            "skills_list_skills" => crate::operations::list_skills::execute(args),
            "skills_search_skills" => crate::operations::search_skills::execute(args),
            _ => Err(McpError::ToolNotFound(name.to_string()).into()),
        }
    }
}

/// Build an MCP tool definition from a params type's derived JSON schema.
/// The schemars `$schema` key is stripped — MCP clients expect a plain
/// JSON Schema fragment, not a top-level draft declaration.
fn tool_def<T: JsonSchema>(name: &str, description: &str) -> Value {
    let mut schema = serde_json::to_value(schema_for!(T)).unwrap_or(Value::Null);
    if let Some(obj) = schema.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }
    serde_json::json!({
        "name": name,
        "description": description,
        "inputSchema": schema,
    })
}
