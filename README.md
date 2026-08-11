# skills-mcp

An MCP server that gives an agent a local library of Agent Skills: reusable
how-to guides and playbooks, stored as `SKILL.md` files on disk.

## Purpose and scope

This server turns a directory of skill files into MCP tools. It finds the skill
directories, parses each `SKILL.md`, and lets a caller read, search, create,
update and delete them. The files are the source of truth. An editor, a script
or a version-control checkout can change them, and this server reads the result
on the next call.

It owns:

- Discovery of the skill roots, and of the skill directories in them.
- The `SKILL.md` format: YAML frontmatter plus a markdown body.
- The six `skills_*` tools and the JSON they return.
- Name validation, so a tool call cannot reach a path outside a configured root.

It does not own these concerns, and refuses them:

- **The MCP protocol.** The handshake, the framing, version negotiation, the
  command line and the error shape all come from
  [mcp-core](https://github.com/adelie-ai/mcp-core).
- **Semantic recall.** Search is a case-insensitive substring match. Ranking,
  embeddings and relevance belong to the calling agent's knowledge base.
- **Running a skill.** The server returns text and the names of the attachment
  files. The agent runs `scripts/run.py` with its own tools.
- **Sync, sharing and history.** Point git or a file-sync tool at the skill root.
- **Authentication and multiple tenants.** The server speaks stdio to one local
  client, and reads the files that client's own user can read.

## How a skill is stored

One directory per skill, under a root:

```text
<root>/<name>/SKILL.md          the skill itself
<root>/<name>/scripts/run.py    every other file is an attachment
```

`SKILL.md` starts with a YAML frontmatter block between `---` lines. The
markdown body follows it:

```markdown
---
name: release-checklist
description: Steps to cut and publish a release. Use this before you tag.
tags: [release, checklist]
---

1. Run the gate.
2. Tag the commit.
```

`name` and `description` are required, `tags` is optional. `name` must equal the
directory name, and must be one path component: no `/`, no `\`, and not `.` or
`..`.

Every other file in the skill directory is reported as an attachment, named by
its path relative to that directory (`scripts/run.py`). The server lists the
attachments. It does not read them and it does not run them. The walk stops four
levels down.

A skill whose `SKILL.md` has no frontmatter block, or which the server cannot
read, is skipped with a warning. The other skills still return.

## Where skills are read from and written to

A read searches these roots in order, and skips a root that does not exist:

| Order | Root |
|---|---|
| 1 | Each colon-separated entry of `$SKILLS_MCP_ROOTS`, left to right |
| 2 | `~/.agents/skills` |
| 3 | `~/.claude/skills` |
| 4 | The write root, if the list does not hold it already |

A `~` or a `$VAR` in `$SKILLS_MCP_ROOTS` is expanded. The first root that holds a
matching directory wins, so an entry in `$SKILLS_MCP_ROOTS` shadows a skill of
the same name in `~/.agents/skills`.

A write goes to one root only: `$SKILLS_MCP_WRITE_ROOT`, or `~/.agents/skills`
when that variable is not set. `skills_create_skill` creates the root if it is
missing. Set `$SKILLS_MCP_WRITE_ROOT` when the default root is read-only, for
example when a package manager owns `~/.agents/skills`.

## MCP tools

| Tool | What it does |
|---|---|
| `skills_create_skill` | Write a new skill to the write root. Fails if any root already holds that name. |
| `skills_get_skill` | Read one skill by name: frontmatter, body, attachment names, and its path. |
| `skills_update_skill` | Change the description, body, tags or name in place. `new_name` renames the directory. |
| `skills_delete_skill` | Remove the whole skill directory, attachments included. |
| `skills_list_skills` | List every skill in every root. Filter with `tags`. |
| `skills_search_skills` | Case-insensitive substring search of name, description, tags and body. Filter with `tags`. |

`skills_list_skills` and `skills_search_skills` leave the absolute `path` and
`root` out of each result, to save tokens and to keep the host layout out of the
model's context. Pass `include_paths: true` to get both fields back.
`skills_get_skill` always reports the path, because an agent opens an attachment
by path.

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

Build the binary:

```sh
cargo build --release
```

Register it with an MCP client. The server speaks stdio, which is the default
transport, so `serve` needs no flag:

```sh
claude mcp add skills -- /path/to/skills-mcp serve
```

Point it at your own skill directories:

```sh
SKILLS_MCP_ROOTS=/srv/team-skills:$HOME/my-skills \
SKILLS_MCP_WRITE_ROOT=$HOME/my-skills \
  ./target/release/skills-mcp serve
```

`serve` also accepts `--transport`, `--host`, `--port` and `--socket-path` from
`mcp-core`, and `--mode` as a back-compatible alias of `--transport`. This server
enables stdio only, so `--transport websocket` and `--transport unix` are refused
with a configuration error.

## Repository layout

- `src/main.rs` - the binary. Hands the config and the service to `mcp_core::run_simple`.
- `src/lib.rs` - `build_service()` for in-process hosting, and `server_config()`, which holds the model-facing `instructions` blurb.
- `src/service.rs` - the `McpService` implementation: the tool list, and dispatch to one operation.
- `src/params.rs` - one typed parameter struct per tool. `schemars` derives each tool's JSON Schema from these.
- `src/operations/` - one module per tool. Each parses its parameters and calls `repo`.
- `src/repo.rs` - the on-disk format: root discovery, name validation, parsing, atomic writes, search.
- `src/error.rs` - the domain error types.

## Development

The toolchain is pinned in `rust-toolchain.toml`. The crate is edition 2024.

```sh
just check        # format, clippy, build and test, with the default features
just check-otel   # clippy, build and test, with the otel feature
just check-all    # both; this is what the pre-push hook runs
just install-hooks
```

`Cargo.toml` denies warnings, so a warning fails the build in both
configurations.

See [AGENTS.md](AGENTS.md) for the coding conventions and this repo's own rules.
