#![deny(warnings)]

// Typed parameter structs for each MCP tool. Schemars derives JSON Schema
// from these so list_tools doesn't have to maintain hand-rolled schemas
// in lockstep with the operation argument parsers.

use crate::db::SkillKind;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CreateSkillParams {
    /// Unique human-friendly name for this skill. Must not be empty.
    pub name: String,
    /// Type of skill: 'code' for executable code snippets, 'howto' for natural-language instructions.
    pub kind: SkillKind,
    /// The actual code or natural-language instructions. Must not be empty.
    pub content: String,
    /// Programming language for kind=code (e.g. 'python', 'rust', 'bash'). Optional for kind=howto.
    #[serde(default)]
    pub language: Option<String>,
    /// Short one-line description of what this skill does.
    #[serde(default)]
    pub description: Option<String>,
    /// Arbitrary tags for filtering and discovery.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct GetSkillParams {
    /// The UUID id or exact name of the skill to retrieve.
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct UpdateSkillParams {
    /// The UUID id or exact name of the skill to update.
    pub id: String,
    /// New unique name for the skill.
    #[serde(default)]
    pub name: Option<String>,
    /// New kind for the skill.
    #[serde(default)]
    pub kind: Option<SkillKind>,
    /// Replacement content. Must not be empty.
    #[serde(default)]
    pub content: Option<String>,
    /// New programming language. Pass null to clear.
    #[serde(default, deserialize_with = "deserialize_some")]
    pub language: Option<Option<String>>,
    /// New short description. Pass null to clear.
    #[serde(default, deserialize_with = "deserialize_some")]
    pub description: Option<Option<String>>,
    /// Replacement tag list (replaces existing tags entirely).
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct DeleteSkillParams {
    /// The UUID id or exact name of the skill to delete.
    pub id: String,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ListSkillsParams {
    /// If provided, return only skills of this kind.
    #[serde(default)]
    pub kind: Option<SkillKind>,
    /// If provided, return only skills that have at least one of these tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SearchSkillsParams {
    /// Search query string. Matched case-insensitively against all text fields.
    pub query: String,
    /// If provided, restrict search to skills of this kind.
    #[serde(default)]
    pub kind: Option<SkillKind>,
}

/// Custom deserializer that distinguishes "key absent" (None) from "key
/// explicitly set to null" (Some(None)). Used by UpdateSkillParams to let
/// callers clear nullable fields by passing null.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}
