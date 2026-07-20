#![deny(warnings)]
#![recursion_limit = "256"]

// Library crate for skills-mcp

pub mod error;
pub mod operations;
pub mod params;
pub mod repo;
pub mod service;

use mcp_core::ServerConfig;

pub use service::SkillsService;

/// Construct the skills service with built-in defaults, for in-process (compiled-in) hosting.
/// Root directories are resolved lazily per call from SKILLS_MCP_ROOTS / SKILLS_MCP_WRITE_ROOT.
pub fn build_service() -> SkillsService {
    SkillsService
}

/// Build the [`ServerConfig`] for skills-mcp: server identity plus the
/// model-facing `instructions` blurb emitted in the MCP `initialize` response.
///
/// Why a library fn instead of inline in `main`: the daemon uses `instructions`
/// as this server's searchable description, so it is discovery-critical and
/// worth pinning. Exposing it here lets tests assert the blurb and transport
/// wiring without launching the binary.
pub fn server_config() -> ServerConfig {
    ServerConfig::new("skills-mcp", env!("CARGO_PKG_VERSION"))
        .instructions(
            "Local library of Anthropic Agent Skills: reusable how-to guides and playbooks \
             stored as SKILL.md files (YAML frontmatter plus a markdown body) on disk. Reach \
             for this whenever you need a saved procedure or want to capture one - to recall \
             how to do a recurring task, look up a documented workflow, or save and update an \
             instruction set for later reuse. Discover with skills_search_skills and \
             skills_list_skills, read a full skill with skills_get_skill, and manage them with \
             skills_create_skill, skills_update_skill, and skills_delete_skill. Skills are read \
             from $SKILLS_MCP_ROOTS plus ~/.agents/skills and ~/.claude/skills; writes go to \
             $SKILLS_MCP_WRITE_ROOT (default ~/.agents/skills).",
        )
        .without_websocket()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::McpService;

    #[test]
    fn build_service_exposes_tools() {
        let svc = build_service();
        assert!(
            !svc.tools().is_empty(),
            "skills build_service() must expose at least one tool"
        );
    }
}
