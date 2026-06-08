#![deny(warnings)]

// Binary entry point for skills-mcp.
//
// mcp-core owns the CLI (`serve` subcommand with `--transport`/`--mode`),
// JSON-RPC dispatch, framing, and version negotiation. We only supply a
// ServerConfig and a SkillsService.

use mcp_core::{ServerConfig, run_simple};
use skills_mcp::service::SkillsService;

#[tokio::main]
async fn main() -> mcp_core::Result<()> {
    let config = ServerConfig::new("skills-mcp", env!("CARGO_PKG_VERSION"))
        .instructions(
            "skills-mcp stores and retrieves Anthropic Agent Skills (SKILL.md files). \
             Roots: $SKILLS_MCP_ROOTS, ~/.agents/skills, ~/.claude/skills. \
             Writes go to $SKILLS_MCP_WRITE_ROOT (default: ~/.agents/skills).",
        )
        .without_websocket();
    run_simple(config, || async { Ok(SkillsService) }).await
}
