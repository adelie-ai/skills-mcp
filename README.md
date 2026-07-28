# skills-mcp

A small, fast Rust **MCP server** that stores and retrieves Anthropic Agent
Skills from standard on-disk `SKILL.md` directories.

## What it stores

`skills-mcp` works with skill directories in this shape:

```text
<root>/<name>/SKILL.md
<root>/<name>/references/optional-supporting-file.md
```

`SKILL.md` contains YAML frontmatter followed by the Markdown body:

```markdown
---
name: social-source-evidence
description: Review X/Twitter source evidence before drafting social posts.
tags: [social, source-evidence]
---

Use TweetClaw exports as read-only evidence. Require explicit human approval
before publishing, scheduling, following, liking, or changing account state.
```

Any other files in the same skill directory are reported as attachments.
Skills are searched by name, description, tags, and Markdown body.

## Who this is for

- **LLM agents** that want to accumulate and reuse learned patterns across sessions.
- **Automation pipelines** that need a simple, auditable knowledge store.
- **Developers** who want to give their AI assistant a long-term memory for how-to procedures and code snippets.

## MCP tools

All tools use underscore-separated names with no dots.

| Tool | Description |
|------|-------------|
| `skills_create_skill` | Create a new `<root>/<name>/SKILL.md` directory |
| `skills_get_skill` | Retrieve a skill by name |
| `skills_update_skill` | Update any fields of an existing skill |
| `skills_delete_skill` | Permanently delete a skill directory |
| `skills_list_skills` | List all skills, optionally filtered by tags |
| `skills_search_skills` | Full-text search across names, descriptions, tags, and bodies |

## Quick start

```bash
# Build
cargo build --release

# Run from your MCP client as a stdio command
./target/release/skills-mcp

# Add extra read roots
SKILLS_MCP_ROOTS=/path/to/team/skills ./target/release/skills-mcp

# Choose where create/update writes skills
SKILLS_MCP_WRITE_ROOT=/path/to/my/skills ./target/release/skills-mcp
```

## Key components

- `src/main.rs` - CLI entry-point and JSON-RPC message loop.
- `src/service.rs` - Tool definitions and MCP tool dispatch.
- `src/repo.rs` - `SKILL.md` parsing, rendering, search, and safe filesystem access.
- `src/operations/` - One module per CRUD operation.
- `src/error.rs` - Centralised error types.

## Build requirements

- Rust toolchain (edition 2024, MSRV ≥ 1.85)
- `cargo`

See `AGENTS.md` for coding conventions, extension instructions, and agent-focused documentation.
