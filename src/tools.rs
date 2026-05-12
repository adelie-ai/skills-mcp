#![deny(warnings)]

// Tool registry: MCP tool definitions and dispatch

use crate::db::SkillDb;
use crate::error::{McpError, Result};
use crate::params::{
    CreateSkillParams, DeleteSkillParams, GetSkillParams, ListSkillsParams, SearchSkillsParams,
    UpdateSkillParams,
};
use schemars::{JsonSchema, schema_for};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Holds all tool definitions and the shared database.
pub struct ToolRegistry {
    db: Arc<Mutex<SkillDb>>,
}

impl ToolRegistry {
    /// Create a new registry backed by the database at `db_path`.
    pub fn new(db_path: &Path) -> Result<Self> {
        let db = SkillDb::open(db_path)?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
        })
    }

    /// Return all tool definitions in MCP JSON schema format.
    pub fn list_tools(&self) -> Value {
        serde_json::json!([
            tool_def::<CreateSkillParams>(
                "skills_create_skill",
                "Create a new skill entry. A skill is either a reusable code snippet (kind=code) or a natural-language how-to guide for an LLM agent (kind=howto). Returns the created skill as JSON including its assigned id."
            ),
            tool_def::<GetSkillParams>(
                "skills_get_skill",
                "Retrieve a single skill by its UUID id or by its unique name. Returns the full skill object as JSON."
            ),
            tool_def::<UpdateSkillParams>(
                "skills_update_skill",
                "Update one or more fields of an existing skill. Only the fields you provide are changed; omitted fields are left unchanged. Pass null for 'language' or 'description' to clear those fields. Returns the updated skill as JSON."
            ),
            tool_def::<DeleteSkillParams>(
                "skills_delete_skill",
                "Permanently delete a skill by its UUID id or exact name. Returns a confirmation message."
            ),
            tool_def::<ListSkillsParams>(
                "skills_list_skills",
                "List all skills in the knowledge base, optionally filtered by kind and/or tags. Returns a JSON array of skill objects sorted by creation time."
            ),
            tool_def::<SearchSkillsParams>(
                "skills_search_skills",
                "Full-text search across all skills. Matches (case-insensitive) against name, description, content, tags, and language. Returns a JSON array of matching skill objects sorted by creation time."
            ),
        ])
    }

    /// Dispatch a tool call to the appropriate operation.
    pub async fn execute_tool(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "skills_create_skill" => {
                let mut db = self.db.lock().await;
                crate::operations::create_skill::execute(args, &mut db)
            }
            "skills_get_skill" => {
                let db = self.db.lock().await;
                crate::operations::get_skill::execute(args, &db)
            }
            "skills_update_skill" => {
                let mut db = self.db.lock().await;
                crate::operations::update_skill::execute(args, &mut db)
            }
            "skills_delete_skill" => {
                let mut db = self.db.lock().await;
                crate::operations::delete_skill::execute(args, &mut db)
            }
            "skills_list_skills" => {
                let db = self.db.lock().await;
                crate::operations::list_skills::execute(args, &db)
            }
            "skills_search_skills" => {
                let db = self.db.lock().await;
                crate::operations::search_skills::execute(args, &db)
            }
            _ => Err(McpError::ToolNotFound(name.to_string()).into()),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        // Default to the standard db path
        let path = default_db_path();
        Self::new(&path).expect("Failed to open default skills db")
    }
}

/// Returns the default database file path: `~/.skills-mcp/skills.json`.
pub fn default_db_path() -> std::path::PathBuf {
    shellexpand::full("~/.skills-mcp/skills.json")
        .map(|s| std::path::PathBuf::from(s.as_ref()))
        .unwrap_or_else(|_| std::path::PathBuf::from(".skills-mcp/skills.json"))
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
