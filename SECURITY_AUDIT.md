# Security Audit — skills-mcp

**Date:** 2026-03-31
**Scope:** Skills/prompts MCP server

---

## Medium Severity

### 1. Shell Expansion on DB Path (MEDIUM)

**File:** `src/db.rs:90-94`

`shellexpand::full()` is used on the database path. If the path comes from `--db-path` CLI arg and an attacker controls the environment, `$(...)` syntax could be expanded.

**Recommendation:** Use `shellexpand::tilde()` instead of `full()` to only expand `~`.

---

## Positive Findings

- No shell command execution
- No `unsafe` code
- SQLite storage with parameterized queries
