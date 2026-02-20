FROM rust:1.93 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/skills-mcp /usr/local/bin/skills-mcp
ENTRYPOINT ["skills-mcp", "serve", "--mode", "stdio"]
