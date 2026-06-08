#![deny(warnings)]

// McpService implementation for skills-mcp.
//
// All protocol concerns (JSON-RPC dispatch, version negotiation, isError
// wrapping, framing) are owned by mcp-core. This module only describes the
// tools and executes them.

use mcp_core::{CallError, McpService, ToolDef, ToolReply, async_trait};
use serde_json::Value;

use crate::error::{McpError, SkillsError, SkillsMcpError};
use crate::operations;
use crate::params::{
    CreateSkillParams, DeleteSkillParams, GetSkillParams, ListSkillsParams, SearchSkillsParams,
    UpdateSkillParams,
};
use schemars::{JsonSchema, schema_for};

/// The skills-mcp McpService. Stateless — every call reads from / writes to
/// disk directly via `crate::repo`.
pub struct SkillsService;

#[async_trait]
impl McpService for SkillsService {
    fn tools(&self) -> Vec<ToolDef> {
        vec![
            tool_def::<CreateSkillParams>(
                "skills_create_skill",
                "Create a new skill on disk as <root>/<name>/SKILL.md with YAML frontmatter \
                 (name, description, optional tags) plus a markdown body. Writes to \
                 ~/.agents/skills, creating it if missing.",
            ),
            tool_def::<GetSkillParams>(
                "skills_get_skill",
                "Read a skill by name. Searches all configured roots (~/.agents/skills, \
                 ~/.claude/skills, and any in $SKILLS_MCP_ROOTS) and returns the parsed \
                 frontmatter, body, absolute path, and a list of attachment filenames in \
                 the skill directory.",
            ),
            tool_def::<UpdateSkillParams>(
                "skills_update_skill",
                "Modify an existing skill in place. Only the fields you set are changed. \
                 If `new_name` is provided the skill directory is renamed.",
            ),
            tool_def::<DeleteSkillParams>(
                "skills_delete_skill",
                "Permanently delete a skill directory (SKILL.md and any attachments) by name.",
            ),
            tool_def::<ListSkillsParams>(
                "skills_list_skills",
                "List every skill across all configured roots. Returns name, description, \
                 tags, and attachment filenames for each. Optionally filter by tag.",
            ),
            tool_def::<SearchSkillsParams>(
                "skills_search_skills",
                "Case-insensitive full-text search across skill names, descriptions, tags, \
                 and SKILL.md bodies. Optionally restrict to skills carrying at least one \
                 of the supplied tags.",
            ),
        ]
    }

    async fn call_tool(&self, name: &str, args: &Value) -> Result<ToolReply, CallError> {
        match name {
            "skills_create_skill" => dispatch(operations::create_skill::execute(args)),
            "skills_get_skill" => dispatch(operations::get_skill::execute(args)),
            "skills_update_skill" => dispatch(operations::update_skill::execute(args)),
            "skills_delete_skill" => dispatch(operations::delete_skill::execute(args)),
            "skills_list_skills" => dispatch(operations::list_skills::execute(args)),
            "skills_search_skills" => dispatch(operations::search_skills::execute(args)),
            other => Err(CallError::tool(format!("unknown tool: {other}"))),
        }
    }
}

/// Convert a domain `Result<Value>` (whose `Value` is already a
/// `{"content":[...]}` object) into the mcp-core `ToolReply`.
///
/// Domain errors become `isError` content (the model can react to them); JSON
/// serialization failures become `CallError::Internal`.
fn dispatch(result: crate::error::Result<Value>) -> Result<ToolReply, CallError> {
    match result {
        Ok(v) => {
            // The operation functions return a `{"content":[{"type":"text","text":...}]}`
            // shape. Extract the text and hand it to mcp-core as a plain text reply.
            let text = v
                .get("content")
                .and_then(|c| c.get(0))
                .and_then(|b| b.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Ok(ToolReply::text(text))
        }
        Err(e) => {
            // Classify the error so mcp-core can surface it correctly.
            match &e {
                SkillsMcpError::Mcp(McpError::InvalidToolParameters(_)) => {
                    Err(CallError::invalid_params(e.to_string()))
                }
                SkillsMcpError::Skills(SkillsError::InvalidInput(_)) => {
                    Err(CallError::invalid_params(e.to_string()))
                }
                SkillsMcpError::Json(_) => Err(CallError::internal(e.to_string())),
                // NotFound, AlreadyExists, StorageError, ToolNotFound, IO —
                // surfaced as isError content so the model can react.
                _ => Err(CallError::tool(e.to_string())),
            }
        }
    }
}

/// Build a [`ToolDef`] from a params type's derived JSON schema. The schemars
/// `$schema` / `title` keys are stripped — MCP clients expect a plain JSON
/// Schema fragment.
fn tool_def<T: JsonSchema>(name: &str, description: &str) -> ToolDef {
    let mut schema = serde_json::to_value(schema_for!(T)).unwrap_or(Value::Null);
    if let Some(obj) = schema.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }
    ToolDef::new(name, description, schema)
}
