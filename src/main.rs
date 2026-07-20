#![deny(warnings)]

// Binary entry point for skills-mcp.
//
// mcp-core owns the CLI (`serve` subcommand with `--transport`/`--mode`),
// JSON-RPC dispatch, framing, and version negotiation. We only supply a
// ServerConfig and a SkillsService.

use mcp_core::run_simple;
use skills_mcp::{build_service, server_config};

#[tokio::main]
async fn main() -> mcp_core::Result<()> {
    run_simple(server_config(), || async { Ok(build_service()) }).await
}
