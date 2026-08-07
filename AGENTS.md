# Agent Instructions

Shared standards live in [AGENTS.base.md](AGENTS.base.md), which is generated. This file holds the rules specific to this repo.

## Rust Conventions

Apply these consistently. The repo gate in **Overrides and additions to the shared base** is the floor.

### Coding
- `?` for error propagation. Reserve `unwrap` / `expect` for tests and proven invariants. When `expect`ing in production, the message must explain the invariant — not just describe what would be unwrapped.
- Prefer `&str` / `&[T]` in argument position; take ownership only when storing.
- Newtype wrappers for invariant-bearing values (validated ids, paths constrained to a directory, etc.).
- `From` / `Into` for type conversions; don't write `to_*` methods when traits suffice.
- Combinators (`map`, `and_then`, `unwrap_or_else`, `?`) over `match` for short `Option` / `Result` chains. Use `match` when there's branching control flow with side effects.
- Avoid `.clone()` on hot paths. `Arc<T>` for shared immutable, `Arc<Mutex<T>>` / `Arc<RwLock<T>>` for shared mutable.

### `unsafe`
- Don't use `unsafe` unless it's necessary AND you've reasoned about soundness. The bar is high.
- Required cases: `std::env::set_var` / `remove_var` (Rust 2024 edition makes these `unsafe` because libc env-mutation is not threadsafe). Anything else needs a strong justification.
- Every `unsafe` block must have a `// SAFETY:` comment naming the invariant the caller is relying on. No "obvious" unsafe — write the soundness argument down. Example:

  ```rust
  // SAFETY: single-threaded test; unique env-var name; no other code touches it.
  unsafe { std::env::remove_var(&unused); }
  ```

### Testing
- Unit tests colocated as `#[cfg(test)] mod tests {}` in lib files.
- Integration tests in `tests/` next to `Cargo.toml`.
- `#[tokio::test]` for async; `#[tokio::test(flavor = "multi_thread")]` only when explicitly testing concurrent behavior.
- Mock at trait boundaries. For HTTP: `httpmock`. For time: an injected `Clock` trait.
- Determinism: sort outputs before assertion; never depend on hash iteration order.
- `expect("descriptive reason")` over `unwrap()` in tests so failure messages are self-explanatory.
- Test public behavior, not private implementation. If a private fn needs testing, surface as `pub(crate)` with a documented contract.
- Don't hold `std::sync::MutexGuard` across `.await`. Drop the guard explicitly before awaiting — `clippy::await_holding_lock` flags this.

### Generics
- `impl Trait` in argument position for single-bound, single-use parameters.
- Named generics with `where` clauses for multiple bounds, recursion, or readability.
- Avoid generic explosion: 3+ generic parameters usually indicates a missing struct or associated type.
- Prefer `Arc<dyn Trait>` over hand-rolled enum-dispatch when there are many implementors and no perf-critical specialization.
- Trait bounds: keep `Send + Sync + 'static` co-located on the trait def when the trait is only useful in async contexts.

### Error handling
- Library crates: `thiserror` with structured variants.
- Binary crates: `anyhow` with `Context::context()` for narrative.
- **Never pattern-match on error message strings.** Pattern-match on variants. If you find yourself doing `error.to_string().contains("429")`, the upstream type is throwing away structured info that should be preserved.
- Surface enough context in `Display` for debugging without leaking secrets.

### Async
- Don't hold non-async locks (`std::sync::Mutex`, `parking_lot::Mutex`) across `.await`. Drop the guard explicitly, or use `tokio::sync::Mutex` if the lock genuinely needs to span the await.
- `tokio::join!` for independent parallel work; `tokio::try_join!` when both must succeed and the first error should cancel the rest.
- Long-running spawned tasks need cancellation — channel-based or `CancellationToken`. Don't leak.
- Cross-cutting context: `tokio::task_local!`.

### Documentation
- Doc comments (`///`) on every public item.
- Include rationale (`Why:` lines) for non-obvious choices, not just descriptions of behavior.
- Don't narrate PR / issue history in code comments. Reference issues only when the comment captures a non-obvious WHY tied to that issue.

## Overrides and additions to the shared base

Everything in [AGENTS.base.md](AGENTS.base.md) applies to this repo. This section
records only the points where this repo deliberately differs from the base, or adds a
rule the base does not have.

### 3.1 The gate for this repo (addition)

The `adelie-ai` repos have no CI. The gate is local and the author runs it:

- `just check` - format, clippy, build and test with default features.
- `just check-otel` - clippy, build and test with the `otel` feature.
- `just check-all` - both. This is what the pre-push hook runs; wire it with
  `just install-hooks`.

Warnings are denied mechanically by the `[lints]` table in `Cargo.toml`, so `cargo build`,
`cargo test`, and `cargo clippy` each hard-fail on a warning, in both configurations.

### 4.3 Branch and pull request - merge when green (override, weaker than the base)

The base opens a pull request and waits for the user. In these repos the merge is delegated:
merge your own pull request as soon as it is green and independently shippable. Green here
means more than a clean build. The gate above passed, the tests cover the new behavior and
not only the absence of a panic, the security pass is done, and the change stands on its own.
Assign `dspadea` with `gh pr edit --add-assignee` and verify it; a review request from the
same account no-ops without an error, so never report a pull request as review-requested.
When in doubt, hold.

### 4.4 Worktrees - the group convention (addition)

Put the worktree at `.worktrees/<repo>/issue-N-slug/` under the group directory, on a branch
that mirrors the slug. Before you run tasks in parallel worktrees, look for shared files,
shared `Cargo.toml` dependency edits, and shared migration ordinals. Serialize the work where
they overlap, and tell each parallel agent the scope it owns.

### 6.1 Dependencies - the group's scan workflow (addition)

Base rule 6.1 sets the policy, including that a high or critical advisory blocks the change.
This group runs it with its own tooling:

1. Add the dependency (`cargo add <crate>`). This writes the lockfile but does not build.
2. Scan the updated lockfile with the `cve-mcp` server's `scan_packages` tool, or with
   `cargo audit`. Pass every (name, version, ecosystem) tuple.
3. Build only after the scan is clean, or after you have accepted the findings in writing.

### 9.1 Tracker for this project

GitHub Issues on `github.com/adelie-ai/skills-mcp`, together with the shared `adelie-ai` project
board `Adelie AI Roadmap` (project number 1). Manage entries with the `gh` CLI
(`gh issue create`, `gh issue list`, `gh issue edit`, `gh pr create`). Put a new issue on the
board with `gh project item-add 1 --owner adelie-ai --url <issue-url>`, which lands it in
Todo. The board states are Todo, In Progress, and Done.
