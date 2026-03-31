# Security Audit — skills-mcp

**Date:** 2026-03-31
**Scope:** Skills/prompts MCP server

---

## High Severity

### 1. Unbounded Memory Allocation from Content-Length

**File:** `src/transport.rs` (shared pattern with tasks-mcp/timeclock-mcp)

No upper bound check on Content-Length before buffer allocation.

**Recommendation:** Add maximum Content-Length check (e.g. 10 MiB).

---

## Medium Severity

### 2. Shell Expansion on DB Path

**File:** `src/db.rs:90-94`

`shellexpand::full()` is used on the database path. If the path comes from `--db-path` CLI arg and an attacker controls the environment, `$(...)` in env vars could be expanded.

**Recommendation:** Use `shellexpand::tilde()` instead of `full()` to only expand `~`, not environment variables or command substitutions.

---

### 3. expect() on User-Controlled Data

**File:** `src/db.rs:199, 232`

```rust
let skill = self.skills.get_mut(&id).expect("skill must exist");
```

Panics crash the server. While logic should prevent this, `expect()` on operations reachable from user input is fragile.

**Recommendation:** Return an error instead of panicking.

---

## Positive Findings

- No shell command execution
- No `unsafe` code
- SQLite storage with parameterized queries
