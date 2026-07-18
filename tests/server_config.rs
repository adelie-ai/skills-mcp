#![deny(warnings)]

// Server-level discovery metadata.
//
// The daemon captures each MCP server's `initialize` `instructions` string and
// uses it as that server's *searchable* description. These tests pin that the
// blurb exists, is non-empty, and carries the four things a discovery hit needs:
// what the server is, when to reach for it (implied by the tool verbs), the key
// tools by name, and where skills are read/written.

use skills_mcp::server_config;

/// Acceptance: the server advertises a non-empty `instructions` string so the
/// daemon has a description to index.
#[test]
fn server_config_exposes_nonempty_instructions() {
    let cfg = server_config();
    let instructions = cfg
        .instructions
        .as_deref()
        .expect("skills-mcp must set server instructions");
    assert!(
        !instructions.trim().is_empty(),
        "instructions must not be blank"
    );
}

/// Acceptance: the blurb names the format (Agent Skills), the discovery/create
/// tools by name, and both root env vars — so a model that matches the server
/// on a query knows what it holds, which tools to call, and how it is configured.
#[test]
fn server_config_instructions_cover_what_tools_and_config() {
    let instructions = server_config()
        .instructions
        .expect("instructions set on the config");
    for needle in [
        "Agent Skills",
        "skills_search_skills",
        "skills_list_skills",
        "skills_get_skill",
        "skills_create_skill",
        "SKILLS_MCP_ROOTS",
        "SKILLS_MCP_WRITE_ROOT",
    ] {
        assert!(
            instructions.contains(needle),
            "instructions should mention {needle:?}; got: {instructions}"
        );
    }
}
