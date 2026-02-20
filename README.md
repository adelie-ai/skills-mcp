# skills-mcp

A small, fast Rust **MCP server** (plus library) that provides a persistent knowledge base of code snippets and how-to guides for LLM agents.

## What it stores

`skills-mcp` stores two types of entries ("skills"):

- **Code snippets** (`kind: code`) — reusable code in any programming language (Python, Rust, Bash, etc.)
- **How-to guides** (`kind: howto`) — natural-language step-by-step instructions an LLM agent can record and replay (e.g. "run this tool, then do this for each result…")

Skills are persisted as a JSON file on disk (default: `~/.skills-mcp/skills.json`) and are fully searchable.

## Who this is for

- **LLM agents** that want to accumulate and reuse learned patterns across sessions.
- **Automation pipelines** that need a simple, auditable knowledge store.
- **Developers** who want to give their AI assistant a long-term memory for how-to procedures and code snippets.

## MCP tools

All tools use underscore-separated names with no dots.

| Tool | Description |
|------|-------------|
| `skills_create_skill` | Create a new code snippet or how-to guide |
| `skills_get_skill` | Retrieve a skill by id or name |
| `skills_update_skill` | Update any fields of an existing skill |
| `skills_delete_skill` | Permanently delete a skill |
| `skills_list_skills` | List all skills, optionally filtered by kind/tags |
| `skills_search_skills` | Full-text search across all fields |

## Quick start

```bash
# Build
cargo build --release

# Run in stdio mode (for VS Code / local MCP clients)
./target/release/skills-mcp serve --mode stdio

# Run as a WebSocket server
./target/release/skills-mcp serve --mode websocket --host 0.0.0.0 --port 8080

# Custom database location
./target/release/skills-mcp serve --mode stdio --db-path ~/my-skills.json
# or
SKILLS_MCP_DB_PATH=~/my-skills.json ./target/release/skills-mcp serve --mode stdio
```

## Key components

- `src/main.rs` — CLI entry-point and JSON-RPC message loop.
- `src/server.rs` — MCP protocol orchestration (initialize, tool dispatch, shutdown).
- `src/tools.rs` — Tool schemas (MCP JSON) and dispatch to operation modules.
- `src/db.rs` — JSON-file-backed in-memory store (`SkillDb`, `Skill`, `SkillKind`).
- `src/operations/` — One module per CRUD operation, each a thin wrapper around `SkillDb`.
- `src/transport.rs` — STDIN/STDOUT and WebSocket transport (auto-detects newline vs Content-Length framing).
- `src/error.rs` — Centralised error types.

## Build requirements

- Rust toolchain (edition 2024, MSRV ≥ 1.85)
- `cargo`

See `AGENTS.md` for coding conventions, extension instructions, and agent-focused documentation.
