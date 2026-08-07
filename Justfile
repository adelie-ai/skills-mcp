# Run MCP integration tests in Docker.
# Requires: docker, just

set shell := ["bash", "-euo", "pipefail", "-c"]

image := "skills-mcp-tests"
container := "skills-mcp-tests"

# Build the project
build:
  cargo build --release

# Build the test image
build-image:
  docker build -t {{image}} .

# Run the tests in a container (container deleted afterward)
test: build-image
  # Ensure we don't collide with a prior run
  docker rm -f {{container}} >/dev/null 2>&1 || true
  docker run --name {{container}} --rm {{image}}
  docker rm -f {{container}} >/dev/null 2>&1 || true

# Run in stdio mode (for development)
run:
  cargo run -- serve --mode stdio

# Run clippy
lint:
  cargo clippy -- -D warnings

# Run tests
unit-test:
  cargo test

# --- Local verification ("local CI") ---
# Run locally instead of GitHub Actions. `install-hooks` wires `check-all` into a
# git pre-push hook so it runs automatically before every push.
# NOTE: `build`/`test` above are a release build and a Docker integration run.
# `check` needs a fast host-side debug compile, so it uses `rust-build` instead
# of `build`, and the existing `unit-test` instead of `test`.
fmt-check:
  cargo fmt --check
fmt:
  cargo fmt
rust-build:
  cargo build

# The gate for the default feature set.
check: fmt-check lint rust-build unit-test

# The gate for the `otel` feature set. This crate ships two configurations
# (mcp-core#40), so both must pass before a push.
check-otel: lint-otel build-otel test-otel
lint-otel:
  cargo clippy --all-targets --features otel -- -D warnings
build-otel:
  cargo build --features otel
test-otel:
  cargo test --features otel

# Every configuration this crate ships in. This is what the pre-push hook runs.
check-all: check check-otel

premerge:
  git fetch origin
  git rebase origin/main
  just check-all
install-hooks:
  git config core.hooksPath .githooks
  @echo "pre-push hook active -- bypass once with: git push --no-verify"
