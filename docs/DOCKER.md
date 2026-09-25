---
title: "Docker"
created: "2026-07-30"
updated: "2026-07-30"
doc_type: "guide"
status: "active"
owner: "rarcane"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# Docker

Docker support lives in `config/Dockerfile` and `docker-compose.yml`.

## Common commands

```bash
just docker-build      # build image
just docker-up         # start compose stack
just docker-down       # stop stack
just docker-rebuild    # rebuild image and recreate container
just docker-logs       # follow logs
just runtime-current   # compare running image with local compose image
```

## Dockerfile pattern

```dockerfile
# syntax=docker/dockerfile:1.7
FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,id=rarcane-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rarcane-cargo-target,target=/app/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked && rm -rf src

# Build real binary
COPY src/ src/
RUN --mount=type=cache,id=rarcane-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rarcane-cargo-target,target=/app/target,sharing=locked \
    touch src/main.rs && cargo build --release --locked && \
    cp target/release/rarcane /usr/local/bin/rarcane

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/rarcane /usr/local/bin/rarcane
RUN groupadd --gid 1000 rarcane && \
    useradd --uid 1000 --gid rarcane --no-create-home --shell /sbin/nologin rarcane && \
    mkdir -p /data && chown rarcane:rarcane /data

USER 1000:1000
EXPOSE 40110/tcp
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:40110/health || exit 1
CMD ["rarcane", "serve", "mcp"]
```

## docker-compose.yml pattern

```yaml
services:
  rarcane-mcp:
    image: ghcr.io/dinglebear-ai/rarcane:${VERSION:-latest}
    build:
      context: .
      dockerfile: config/Dockerfile
    container_name: rarcane-mcp
    restart: unless-stopped
    user: "${PUID:-1000}:${PGID:-1000}"
    env_file:
      - path: .env
        required: false
    ports:
      - "${RARCANE_MCP_HOST_PORT:-40110}:40110/tcp"
    volumes:
      - ${HOME}/.rarcane:/data
    networks:
      - mcp
    healthcheck:
      test: ["CMD-SHELL", "curl -sf http://localhost:40110/health || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    deploy:
      resources:
        limits:
          memory: 256M
          cpus: '0.5'

networks:
  mcp:
    name: ${DOCKER_NETWORK:-rarcane-mcp}
    external: true
```

Key requirements:
- `container_name` must be unique across your stack.
- Use the `${DOCKER_NETWORK:-mcp}` external network.
- `env_file.required: false` so the container starts without `.env` (relies on `config.toml` defaults).
- Resource limits to prevent runaway memory on homelab.

Create the network if it doesn't exist:

```bash
docker network create mcp
```

## Appdata convention

Local binary and Docker use the same data directory:

| Deployment | Data directory |
|---|---|
| Local binary | `~/.rarcane/` |
| Docker | `/data/` inside container, mounted from `~/.rarcane/` on host |
| Plugin | `$CLAUDE_PLUGIN_DATA` (symlinked to `~/.rarcane/`) |

```rust
fn default_data_dir() -> PathBuf {
    if std::path::Path::new("/.dockerenv").exists() {
        return PathBuf::from("/data");
    }
    dirs::home_dir().unwrap_or_default().join(".rarcane")
}
```

## Docker entrypoint

Every Docker image has an `entrypoint.sh` that runs as root, fixes permissions, hardens secret files, validates upstream credentials only for modes that construct `ArcaneClient`, then drops to UID 1000. Diagnostic commands such as `doctor`, `setup`, `status`, `watch`, and `--help` deliberately remain usable when credentials are missing.

```bash
#!/bin/sh
set -e
DATA_DIR="${DATA_DIR:-/data}"

require_arcane_credentials() {
    [ -n "${RARCANE_API_URL:-}" ] || { echo "FATAL: RARCANE_API_URL is not set" >&2; exit 1; }
    [ -n "${RARCANE_API_KEY:-}" ] || { echo "FATAL: RARCANE_API_KEY is not set" >&2; exit 1; }
}

mkdir -p "${DATA_DIR}"
chown -R 1000:1000 "${DATA_DIR}" 2>/dev/null || true

for f in "${DATA_DIR}/.env" "${DATA_DIR}/auth-jwt.pem"; do
    [ -f "${f}" ] && chmod 600 "${f}" || true
done

case "${1:-}" in
  serve|mcp|call|"")
    require_arcane_credentials
    exec gosu 1000:1000 /usr/local/bin/rarcane "$@"
    ;;
  status|watch|doctor|setup|help|--help|-h|--version|-V|version)
    exec gosu 1000:1000 /usr/local/bin/rarcane "$@"
    ;;
  *)
    exec gosu 1000:1000 "$@"
    ;;
esac
```

Key principles: fail fast for commands that actually need the upstream, keep repair/diagnostic paths available during failures, avoid `eval` around secrets, and use `exec` so PID 1 is the actual service. The checked-in Debian image uses `gosu`; Alpine derivatives should switch both Dockerfile and entrypoint to `su-exec` together.

## Health and auth

- Healthcheck probes `/health`.
- `/mcp` requires auth outside loopback unless explicitly behind a trusted gateway.
- Use `scripts/test-mcp-auth.sh` for bearer auth smoke tests.
- Recreate (don't just restart) the container after editing `.env`:

```bash
docker compose up -d --force-recreate
```

## Build artifacts

`just build-plugin` copies the release binary to both `bin/rarcane` and `plugins/rarcane/bin/rarcane`. The plugin binary path is allowlisted in `scripts/blob-size-allowlist.txt`.

See `docs/PATTERNS.md` §14, §15, §25, §26, §50 for the full Dockerfile, compose, appdata, and entrypoint patterns.
