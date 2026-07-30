---
title: "rarcane Configuration"
created: "2026-07-30"
updated: "2026-07-30"
---

# rarcane Configuration

## Arcane

| Variable | Purpose |
|---|---|
| `RARCANE_API_URL` | Arcane base URL. `/api` is appended when needed. |
| `RARCANE_API_KEY` | Arcane API key. Stored in env/config only. |

## MCP

| Variable | Default | Purpose |
|---|---|---|
| `RARCANE_MCP_HOST` | `127.0.0.1` | HTTP bind host |
| `RARCANE_MCP_PORT` | `40110` | HTTP bind port |
| `RARCANE_MCP_TOKEN` | unset | Static bearer token |
| `RARCANE_MCP_NO_AUTH` | false | Disable auth on loopback only |
| `RARCANE_NOAUTH` | false | Explicit trusted gateway mode |
| `RARCANE_MCP_ALLOWED_HOSTS` | unset | Extra Host header values |
| `RARCANE_MCP_ALLOWED_ORIGINS` | unset | Extra CORS origins |
| `RARCANE_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth` |

## Auth Policy

| State | Condition | Behavior |
|---|---|---|
| `LoopbackDev` | loopback bind or loopback no-auth | no auth, no scopes |
| `TrustedGatewayUnscoped` | `RARCANE_NOAUTH=true` behind a trusted gateway | no local auth or scopes |
| `Mounted` bearer | non-loopback with `RARCANE_MCP_TOKEN` | bearer auth and scope checks |
| `Mounted` OAuth | `RARCANE_MCP_AUTH_MODE=oauth` | OAuth/JWT auth and scope checks |

Use `rarcane setup check` for read-only validation and `rarcane setup repair` to create a local `.env`.

OAuth mode supports one replica on one host because sessions, client
registrations, and signing keys are local. Do not horizontally scale OAuth mode
until those stores are externalized or safely shared.
