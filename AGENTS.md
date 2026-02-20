# AGENTS.md — skills-mcp

This file describes the project structure, conventions, and workflows for AI coding agents working on `skills-mcp`.

---

## What this project is

`skills-mcp` is a Rust **MCP server** (plus library) that exposes a knowledge-base of reusable code snippets and natural-language how-to guides over an MCP tool interface. It is intended for use by **LLM agents** that want to store and retrieve learned patterns, procedures, and code across sessions.

Two kinds of entries are stored:

| Kind | Description |
|------|-------------|
| `code` | A reusable code snippet in a specific programming language (e.g. Python, Rust, Bash) |
| `howto` | A natural-language step-by-step guide an LLM agent can follow (e.g. "run this tool, then do this…") |

---

## Repository layout

```
skills-mcp/
├── Cargo.toml                  # Crate manifest and dependencies
├── Justfile                    # Developer task runner
├── Dockerfile                  # Build + runtime image
├── AGENTS.md                   # This file
├── README.md                   # Human-facing documentation
└── src/
    ├── main.rs                 # CLI entry-point (binary `skills-mcp`)
    ├── lib.rs                  # Library interface and module declarations
    ├── server.rs               # MCP server orchestration (initialize, tool dispatch, shutdown)
    ├── tools.rs                # Tool registry: MCP JSON schemas + dispatch to operations
    ├── transport.rs            # STDIN/STDOUT and WebSocket transport (newline-JSON + Content-Length framing)
    ├── error.rs                # All error types (SkillsMcpError, SkillsError, McpError, TransportError)
    ├── db.rs                   # JSON-file-backed in-memory skill store (Skill, SkillKind, SkillDb)
    └── operations/
        ├── mod.rs              # Module declarations
        ├── create_skill.rs     # Parse args → db.create()
        ├── get_skill.rs        # Parse args → db.get()
        ├── update_skill.rs     # Parse args → db.update()
        ├── delete_skill.rs     # Parse args → db.delete()
        ├── list_skills.rs      # Parse args → db.list()
        └── search_skills.rs    # Parse args → db.search()
```

---

## MCP tools (no dots in names — underscores only)

All tool names use the prefix `skills_` followed by a snake_case verb phrase. **Never use dots in tool names.**

| Tool name | Description |
|-----------|-------------|
| `skills_create_skill` | Create a new code snippet or how-to guide |
| `skills_get_skill` | Retrieve a skill by UUID id or exact name |
| `skills_update_skill` | Update fields of an existing skill (partial update) |
| `skills_delete_skill` | Permanently delete a skill by UUID id or exact name |
| `skills_list_skills` | List all skills, optionally filtered by `kind` and/or `tags` |
| `skills_search_skills` | Full-text search across name, description, content, tags, and language |

All tools return a JSON-RPC result shaped as `{ "content": [{ "type": "text", "text": "<json or message>" }] }`.

---

## Data model

The `Skill` struct (defined in `src/db.rs`) has the following fields:

```
id          String          UUID v4, assigned on create (read-only after creation)
name        String          Unique human-friendly name (required)
kind        SkillKind       "code" or "howto" (required)
language    Option<String>  Programming language for kind=code (e.g. "python", "bash")
description Option<String>  Short one-line description
content     String          The actual code or instructions (required, must not be empty)
tags        Vec<String>     Arbitrary tags for filtering
created_at  String          ISO 8601 timestamp (set on create, read-only)
updated_at  String          ISO 8601 timestamp (updated on every write)
```

---

## Storage

Skills are persisted as a pretty-printed JSON file. The default path is `~/.skills-mcp/skills.json`. This can be overridden with:

- CLI flag: `--db-path <path>`
- Environment variable: `SKILLS_MCP_DB_PATH=<path>`

All writes use an atomic rename (write to `.tmp`, then `rename`) to prevent corruption.

---

## Error handling conventions

- All errors flow through the `SkillsMcpError` top-level enum in `src/error.rs`.
- Use `SkillsError::NotFound` when a skill id/name does not exist.
- Use `SkillsError::AlreadyExists` when a `name` collision occurs on create or rename.
- Use `SkillsError::InvalidInput` for bad parameter values (empty content, unknown kind, etc.).
- Use `McpError::InvalidToolParameters` in operations when a required JSON argument is missing.
- `Result<T>` is the type alias `std::result::Result<T, SkillsMcpError>`.
- Operations must return `Result<serde_json::Value>` — never panic.

---

## Adding a new tool

1. Add the operation function in a new file `src/operations/<verb_noun>.rs` with signature:
   ```rust
   pub fn execute(args: &serde_json::Value, db: &[mut] SkillDb) -> Result<serde_json::Value>
   ```
2. Register the module in `src/operations/mod.rs`.
3. Add the JSON schema entry to `ToolRegistry::list_tools()` in `src/tools.rs`.
4. Add the dispatch arm to `ToolRegistry::execute_tool()` in `src/tools.rs`.
5. Run `cargo clippy -- -D warnings` and `cargo test` to verify.

---

## Naming conventions

- **Tool names**: `skills_<verb>_<noun>` with underscores only (e.g. `skills_create_skill`). No dots.
- **Rust modules**: snake_case matching the operation verb-noun (e.g. `create_skill`).
- **Rust types**: PascalCase (e.g. `SkillDb`, `SkillKind`, `CreateSkillRequest`).
- **JSON field names**: snake_case in all serialised output and input schemas.
- **Error variants**: descriptive PascalCase with a single `String` payload (e.g. `NotFound(String)`).

---

## Transport

The server supports two transports (set with `--mode`):

| Mode | Description |
|------|-------------|
| `stdio` | STDIN/STDOUT, newline-delimited JSON **or** `Content-Length` framed JSON-RPC. Recommended for VS Code and local use. |
| `websocket` | WebSocket on `--host`/`--port`. Recommended for hosted deployments. |

Transport logic is isolated in `src/transport.rs` and must not be mixed with business logic.

---

## Building and running

```bash
# Build release binary
cargo build --release

# Run in stdio mode (for local/VS Code use)
./target/release/skills-mcp serve --mode stdio

# Run with a custom db path
./target/release/skills-mcp serve --mode stdio --db-path /path/to/skills.json

# Run clippy (zero warnings policy)
cargo clippy -- -D warnings

# Run tests
cargo test
```

The `Justfile` has shorthand targets: `just build`, `just run`, `just lint`, `just unit-test`.

---

## Coding standards

- All source files begin with `#![deny(warnings)]`.
- Zero clippy warnings (`all = "deny"` in `[lints.clippy]`).
- No `unwrap()` or `expect()` in non-test code — propagate errors with `?`.
- Keep `main.rs` as a thin wiring layer; business logic belongs in `db.rs` and `operations/`.
- Operations must be pure functions of their inputs + db state; no global mutable state outside `SkillDb`.
- Tests live in `#[cfg(test)]` modules within the relevant source file or in `tests/`.
