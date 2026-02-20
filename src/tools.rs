#![deny(warnings)]

// Tool registry: MCP tool definitions and dispatch

use crate::db::SkillDb;
use crate::error::{McpError, Result};
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
            {
                "name": "skills_create_skill",
                "description": "Create a new skill entry. A skill is either a reusable code snippet (kind=code) or a natural-language how-to guide for an LLM agent (kind=howto). Returns the created skill as JSON including its assigned id.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Unique human-friendly name for this skill. Must not be empty."
                        },
                        "kind": {
                            "type": "string",
                            "enum": ["code", "howto"],
                            "description": "Type of skill: 'code' for executable code snippets, 'howto' for natural-language instructions."
                        },
                        "content": {
                            "type": "string",
                            "description": "The actual code or natural-language instructions. Must not be empty."
                        },
                        "language": {
                            "type": "string",
                            "description": "Programming language for kind=code (e.g. 'python', 'rust', 'bash'). Optional for kind=howto."
                        },
                        "description": {
                            "type": "string",
                            "description": "Short one-line description of what this skill does."
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Arbitrary tags for filtering and discovery."
                        }
                    },
                    "required": ["name", "kind", "content"]
                }
            },
            {
                "name": "skills_get_skill",
                "description": "Retrieve a single skill by its UUID id or by its unique name. Returns the full skill object as JSON.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The UUID id or exact name of the skill to retrieve."
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "skills_update_skill",
                "description": "Update one or more fields of an existing skill. Only the fields you provide are changed; omitted fields are left unchanged. Pass null for 'language' or 'description' to clear those fields. Returns the updated skill as JSON.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The UUID id or exact name of the skill to update."
                        },
                        "name": {
                            "type": "string",
                            "description": "New unique name for the skill."
                        },
                        "kind": {
                            "type": "string",
                            "enum": ["code", "howto"],
                            "description": "New kind for the skill."
                        },
                        "content": {
                            "type": "string",
                            "description": "Replacement content. Must not be empty."
                        },
                        "language": {
                            "type": ["string", "null"],
                            "description": "New programming language. Pass null to clear."
                        },
                        "description": {
                            "type": ["string", "null"],
                            "description": "New short description. Pass null to clear."
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Replacement tag list (replaces existing tags entirely)."
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "skills_delete_skill",
                "description": "Permanently delete a skill by its UUID id or exact name. Returns a confirmation message.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The UUID id or exact name of the skill to delete."
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "skills_list_skills",
                "description": "List all skills in the knowledge base, optionally filtered by kind and/or tags. Returns a JSON array of skill objects sorted by creation time.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["code", "howto"],
                            "description": "If provided, return only skills of this kind."
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "If provided, return only skills that have at least one of these tags."
                        }
                    }
                }
            },
            {
                "name": "skills_search_skills",
                "description": "Full-text search across all skills. Matches (case-insensitive) against name, description, content, tags, and language. Returns a JSON array of matching skill objects sorted by creation time.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query string. Matched case-insensitively against all text fields."
                        },
                        "kind": {
                            "type": "string",
                            "enum": ["code", "howto"],
                            "description": "If provided, restrict search to skills of this kind."
                        }
                    },
                    "required": ["query"]
                }
            }
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
