# syntax=docker/dockerfile:1.7
# =============================================================================
# Dockerfile — Template for building an rmcp-based MCP server
#
# TEMPLATE: Replace all occurrences of "rarcane" with your binary name.
#           The binary name must match Cargo.toml [[bin]] name = "rarcane".
#
# Key substitutions:
#   "rarcane"              → your binary name (e.g. "myservice")
#   "rarcane-cargo-*"      → your cache mount IDs (e.g. "myservice-cargo-*")
#   "arcane-rmcp"          → your systemd/Docker service name
#   EXPOSE 40110            → your port (must match config.toml [mcp] port)
#   HEALTHCHECK port 40110  → same as EXPOSE
#   CMD ["rarcane", ...]   → your binary name
#
# Build stages:
#   builder  — Rust compilation with BuildKit cache mounts (fast incremental builds)
#   runtime  — Minimal Debian image with only the binary + curl for health checks
# =============================================================================

# ── Stage 1: Builder ──────────────────────────────────────────────────────────
# TEMPLATE: Pin to a digest for reproducible builds:
#   docker pull rust:1.97.1-slim-bookworm
#   docker inspect --format '{{index .RepoDigests 0}}' rust:1.97.1-slim-bookworm
FROM rust:1.97.1-slim-bookworm@sha256:99e09cb2284e2ddbb73a995deee3e91783fd04d177602ccf6eab326d778ee777 AS builder

WORKDIR /app

# Keep BuildKit caches architecture-specific. Multi-platform builds execute
# these stages concurrently; shared locked cache IDs serialize the builds and
# sharing target artifacts across architectures can corrupt the cache.
ARG TARGETARCH

# Install build dependencies.
# pkg-config and libssl-dev are needed for openssl-sys (used by reqwest/lab-auth).
# TEMPLATE: Add other system deps your crates need (e.g. libpq-dev for postgres).
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      pkg-config=1.8.1-1 \
      libssl-dev=3.0.20-1~deb12u2 && \
    rm -rf /var/lib/apt/lists/*

# ── Dependency caching layer ──────────────────────────────────────────────────
# Copy only Cargo manifests first and build a stub binary. This lets Docker/
# BuildKit cache the compiled dependencies as a separate layer — subsequent
# builds that only change src/ skip this expensive step entirely.
COPY Cargo.toml Cargo.lock ./
# Strip xtask workspace member — not needed in Docker build context
RUN sed -i 's/, \"xtask\"//g; s/\"xtask\", //g; s/\"xtask\"//g' Cargo.toml

# TEMPLATE: Replace "rarcane" cache IDs with your service name to avoid
#           sharing cache with other projects on the same build host.
RUN --mount=type=cache,id=rarcane-cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rarcane-cargo-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=rarcane-cargo-target-${TARGETARCH},target=/app/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked -p rarcane && rm -rf src

# ── Application build ─────────────────────────────────────────────────────────
# Now copy the real source and build the actual binary.
# The `touch src/main.rs` forces Cargo to re-link even when timestamps haven't
# changed (Docker layer caching can confuse Cargo's incremental build detection).
COPY src/ src/
RUN --mount=type=cache,id=rarcane-cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rarcane-cargo-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=rarcane-cargo-target-${TARGETARCH},target=/app/target,sharing=locked \
    touch src/main.rs && cargo build --release --locked -p rarcane && \
    cp target/release/rarcane /usr/local/bin/rarcane

# ── Stage 3: Runtime ──────────────────────────────────────────────────────────
# Minimal Debian image — no Rust toolchain, no build tools.
# Only ca-certificates (for TLS) and curl (for HEALTHCHECK).
# Pinned to bookworm-slim 2026-05-15 — update quarterly with:
#   docker pull debian:bookworm-slim
#   docker inspect --format '{{index .RepoDigests 0}}' debian:bookworm-slim
FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818

# Install runtime dependencies:
#   ca-certificates — TLS certificate verification for outbound HTTPS calls
#   curl            — used by HEALTHCHECK
#   gosu            — privilege drop in entrypoint.sh (exec as UID 1000:1000)
#
# TEMPLATE: Add other runtime libraries your binary needs (e.g. libssl1.1 if not
#           statically linked). Alpine users: replace gosu with su-exec and update
#           entrypoint.sh accordingly.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates=20230311+deb12u1 \
      curl=7.88.1-10+deb12u15 \
      gosu=1.14-1+b10 && \
    rm -rf /var/lib/apt/lists/*

# TEMPLATE: Replace "rarcane" with your binary name in both COPY and target path.
COPY --from=builder /usr/local/bin/rarcane /usr/local/bin/rarcane

# Wire in the entrypoint.
# TEMPLATE: entrypoint.sh lives at the repo root. It runs as root, fixes /data
#           permissions, validates env vars, then exec's as UID 1000:1000.
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Create a non-root user and group for the service.
# UID/GID 1000 matches the host user convention in docker-compose.yml.
# TEMPLATE: Replace "rarcane" with your service name for the user/group.
RUN groupadd --gid 1000 rarcane && \
    useradd --uid 1000 --gid rarcane --no-create-home --shell /sbin/nologin rarcane && \
    mkdir -p /data && \
    chown rarcane:rarcane /data

# Default log level. Override with RUST_LOG env var in docker-compose or at runtime.
ENV RUST_LOG=info

# TEMPLATE: Add other default env vars your binary needs.
# ENV RARCANE_MCP_HOST=0.0.0.0
# ENV RARCANE_MCP_PORT=3000

# NOTE: No USER directive here — entrypoint.sh drops to UID 1000:1000 via gosu.
# This allows entrypoint.sh to run as root (for chown /data) before execing the binary.
# Pattern §26 + §16 (USER 1000:1000 via entrypoint, not Dockerfile USER directive).

# TEMPLATE: Change to match your config.toml [mcp] port = 3000
EXPOSE 40110/tcp

# Health check hits /health (always unauthenticated in the rarcane design).
# TEMPLATE: Change port to match EXPOSE above.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:40110/health || exit 1

# MCP registry label — used by Claude Code plugin discovery.
# TEMPLATE: Replace with your registry path: "tv.nashost/yourservice-mcp"
LABEL io.modelcontextprotocol.server.name="tv.nashost/arcane-rmcp"

# Entrypoint handles permission setup and privilege drop.
# CMD is the default argument passed to entrypoint.sh.
# TEMPLATE: Replace "rarcane" with your binary name.
ENTRYPOINT ["/entrypoint.sh"]
CMD ["serve", "mcp"]
