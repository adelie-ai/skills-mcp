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
