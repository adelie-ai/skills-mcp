#![deny(warnings)]

// Tool-level discovery metadata.
//
// A tool description is the model-facing text that leads tool selection. These
// tests pin that the two descriptions we sharpen lead with purpose and the
// natural terms a user/model would actually use ("find", "save"), rather than
// opening on mechanism ("case-insensitive full-text search", the on-disk path).
// The other four descriptions (get/update/delete/list) already lead with
// purpose and are intentionally not pinned here.

use mcp_core::McpService;
use skills_mcp::service::SkillsService;

/// Look up a tool's model-facing description by tool name.
fn description_of(tool: &str) -> String {
    SkillsService
        .tools()
        .into_iter()
        .find(|t| t.name == tool)
        .unwrap_or_else(|| panic!("tool {tool} must be exposed"))
        .description
}

/// Acceptance: search leads with the natural "find" framing and mentions
/// keyword search, not a mechanism-first "case-insensitive full-text search".
#[test]
fn search_skills_description_leads_with_find() {
    let d = description_of("skills_search_skills");
    assert!(d.starts_with("Find"), "search should lead with 'Find': {d}");
    assert!(
        d.to_lowercase().contains("keyword"),
        "search should mention keyword search: {d}"
    );
}

/// Acceptance: create leads with saving a reusable skill and uses the natural
/// how-to / playbook vocabulary rather than opening on the on-disk SKILL.md path.
#[test]
fn create_skill_description_leads_with_save() {
    let d = description_of("skills_create_skill");
    let lower = d.to_lowercase();
    assert!(
        lower.starts_with("save"),
        "create should lead with 'Save': {d}"
    );
    assert!(
        lower.contains("how-to") || lower.contains("playbook"),
        "create should use how-to/playbook vocabulary: {d}"
    );
}
