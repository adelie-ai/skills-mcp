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

## Logging

skills-mcp gets its logging, tracing and metrics from `mcp-core`, which installs
them through [adelie-telemetry](https://github.com/adelie-ai/adelie-telemetry).
This section covers what is specific to this server; `mcp-core`'s own README
has the full contract.

### Where it goes, and how much

**stderr, always.** This server speaks stdio, and the transport frames
JSON-RPC on stdout, so a log line there would corrupt the protocol -- this
holds even at `RUST_LOG=trace`.

`RUST_LOG` sets the filter. Unset means `info`.

```sh
RUST_LOG=debug skills-mcp serve
```

### What may appear at each level

| Level | Carries |
|---|---|
| INFO | ids, counts, durations, tool names, a skipped skill's own directory name. **Never a path.** |
| DEBUG | tool arguments, and a skipped skill's full path and error detail. |

`repo::list_all` skips a skill it cannot read (a missing frontmatter block, an
unreadable file) instead of failing the whole listing. The skill's directory
name is logged at WARN as an identifier, the same class of value as a tool
name. The full path is not: it resolves through `~/.agents/skills` and
`~/.claude/skills`, so it carries the operator's home directory, and the
underlying error can quote a snippet of the file's own content on a parse
failure. Both move to DEBUG instead. `skills.skipped_entries`, labelled by a
bounded `reason` (`read_failed`, `invalid_frontmatter`, or `other`), counts
these regardless of the log level -- see Metrics below.

### Metrics

`mcp-core`'s dispatch layer already records a tool-call counter and a latency
histogram, by tool name and outcome, for every call this server handles; see
`mcp-core`'s README for the full list. This server adds one metric of its own:

| Metric | Labels | Meaning |
|---|---|---|
| `skills.skipped_entries` | `reason` | A skill `list_all`/`search` could not read, by why. |

### Exporting to a collector

Off by default. Turn it on with the `otel` feature:

```sh
cargo build --features otel
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 ./target/debug/skills-mcp serve
```

With the feature off, no opentelemetry crate is resolved at all -- `cargo
tree` on a default build shows none. With it on, traces, metrics and log
records export over the standard `OTEL_EXPORTER_OTLP_*` / `OTEL_RESOURCE_*`
environment variables; there are no server-specific flags or variables. See
`mcp-core`'s README for the full variable list and
[adelie-telemetry](https://github.com/adelie-ai/adelie-telemetry)'s for
transport and TLS details.

With no collector configured, the metrics registry still accumulates and
still writes a periodic summary to stderr, so a plain `cargo install` build
reports real numbers without any extra setup.

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
