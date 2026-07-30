---
title: "Environment Variables"
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
  - "src/config.rs"
last_reviewed: "2026-05-15"
---

# Environment variables

The template uses `RARCANE_*` variables. Rename the prefix when adapting the template.

## Upstream service

| Variable | Purpose |
|---|---|
| `RARCANE_API_URL` | Upstream API base URL used by `ArcaneClient`. Required. |
| `RARCANE_API_KEY` | Upstream API key or token. Keep secret. Required. |

## MCP HTTP server

| Variable | Default | Purpose |
|---|---:|---|
| `RARCANE_MCP_HOST` | `127.0.0.1` | Bind host for HTTP transport. Set `0.0.0.0` only with bearer, OAuth, or trusted-gateway auth configured. |
| `RARCANE_MCP_PORT` | `40110` | Bind port for HTTP transport. |
| `RARCANE_MCP_NO_AUTH` | `false` | Disable local auth for loopback development only. |
| `RARCANE_NOAUTH` | `false` | Trusted-gateway no-auth mode for non-loopback deployments. |
| `RARCANE_MCP_TOKEN` | unset | Static bearer token. Required for bearer-only mounted HTTP. |
| `RARCANE_MCP_ALLOWED_HOSTS` | unset | Extra accepted Host header values (comma-separated). |
| `RARCANE_MCP_ALLOWED_ORIGINS` | unset | Extra CORS origins (comma-separated). |
| `RARCANE_MCP_PUBLIC_URL` | unset | Public URL used for OAuth metadata endpoints. |
| `RARCANE_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth`. |

## OAuth mode

Only required when `RARCANE_MCP_AUTH_MODE=oauth`:

| Variable | Purpose |
|---|---|
| `RARCANE_MCP_GOOGLE_CLIENT_ID` | Google OAuth client ID. |
| `RARCANE_MCP_GOOGLE_CLIENT_SECRET` | Google OAuth client secret. |
| `RARCANE_MCP_AUTH_ADMIN_EMAIL` | Initial/admin email allowed by the OAuth flow. |

## Docker runtime

| Variable | Purpose |
|---|---|
| `PUID` | UID to run the container as (default: 1000). |
| `PGID` | GID to run the container as (default: 1000). |
| `DOCKER_NETWORK` | Docker network name (default: `mcp`). |
| `RARCANE_MCP_VERSION` | Image tag to pull (default: `latest`). |

## Logging

| Variable | Arcane | Purpose |
|---|---|---|
| `RUST_LOG` | `info,rmcp=warn` | Tracing filter. |
| `NO_COLOR` | `1` | Disable ANSI color in console logs. |
| `FORCE_COLOR` | `1` | Force ANSI color even when stderr is not a TTY. |

## `.env` file structure

```bash
# .env — secrets and URLs ONLY
RARCANE_API_URL=https://rarcane.internal/api
RARCANE_API_KEY=your_api_key_here

# MCP auth
RARCANE_MCP_TOKEN=your_bearer_token_here

# OAuth (only when auth_mode=oauth in config.toml)
# RARCANE_MCP_GOOGLE_CLIENT_ID=...
# RARCANE_MCP_GOOGLE_CLIENT_SECRET=...

# Docker runtime
PUID=1000
PGID=1000
DOCKER_NETWORK=mcp
RUST_LOG=info
```

## Safety

`.env` and secret-bearing `.env.*` files are ignored and blocked from commits.
Use the tracked `.env.example` only as a placeholder template.

Non-secret settings (host, port, auth mode, TTLs) go in `config.toml`, not `.env`. See `docs/CONFIG.md` for the full split.

Generate a bearer token:

```bash
just gen-token
# or: openssl rand -hex 32
```

See `docs/CONFIG.md` for the config loading pattern and auth policy details.
