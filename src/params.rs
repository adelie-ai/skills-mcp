#![deny(warnings)]

// Typed parameter structs for each MCP tool. Schemars derives JSON Schema
// from these so list_tools doesn't have to maintain hand-rolled schemas
// in lockstep with the operation argument parsers.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ListSkillsParams {
    /// If provided, return only skills whose frontmatter tags include at least one of these.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Include the absolute on-disk `path`/`root` fields in the result. Off by
    /// default to save tokens and avoid leaking the filesystem layout.
    #[serde(default)]
    pub include_paths: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct GetSkillParams {
    /// The name of the skill (the SKILL.md frontmatter `name` and the directory name).
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CreateSkillParams {
    /// Unique skill name. Becomes the directory name under the write root.
    pub name: String,
    /// One- or two-sentence description telling an agent when to use this skill. Stored as the frontmatter `description`.
    pub description: String,
    /// Markdown body of SKILL.md (everything after the YAML frontmatter block). Must not be empty.
    pub content: String,
    /// Optional frontmatter tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct UpdateSkillParams {
    /// The current name of the skill to update.
    pub name: String,
    /// New name. If set, the skill's directory is renamed.
    #[serde(default)]
    pub new_name: Option<String>,
    /// Replacement frontmatter description.
    #[serde(default)]
    pub description: Option<String>,
    /// Replacement markdown body.
    #[serde(default)]
    pub content: Option<String>,
    /// Replacement tag list (replaces existing tags entirely).
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct DeleteSkillParams {
    /// The name of the skill to delete. Removes the entire skill directory.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SearchSkillsParams {
    /// Case-insensitive search across name, description, tags, and body.
    pub query: String,
    /// If provided, restrict matches to skills that have at least one of these tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Include the absolute on-disk `path`/`root` fields in each result. Off by
    /// default to save tokens and avoid leaking the filesystem layout.
    #[serde(default)]
    pub include_paths: bool,
}
