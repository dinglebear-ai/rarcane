---
title: "Architecture"
created: "2026-07-30"
updated: "2026-07-30"
---

# Architecture

`rarcane` is an upstream-client MCP server and CLI for Arcane. It deliberately
does not duplicate Arcane's REST API or web UI.

## Layers

```text
ArcaneClient  (src/arcane.rs)     network requests and upstream decoding
      |
ArcaneService (src/app.rs)        validation, paths, parameters, response shaping
      |
      +-- MCP (src/mcp/)          protocol and scope enforcement
      +-- CLI (src/cli.rs)        argument parsing and JSON output
```

Business logic belongs in `ArcaneService`; MCP and CLI are parsing/delegation
shims. `src/actions.rs` is the canonical registry for Arcane action metadata.

## Runtime modules

| Path | Responsibility |
|---|---|
| `src/arcane.rs` | Authenticated HTTP client for the upstream Arcane API. |
| `src/app.rs` | Shared dispatch, validation, path/query/body construction, and response normalization. |
| `src/actions.rs` | Action/subaction metadata and input parsing. |
| `src/mcp/tools.rs` | MCP tool dispatch into the service. |
| `src/mcp/schemas.rs` | MCP input schema generated from action metadata. |
| `src/mcp/rmcp_server.rs` | MCP tools, resource, prompt, and scope enforcement. |
| `src/server.rs` | Application state and HTTP authentication policy. |
| `src/server/routes.rs` | HTTP route composition and request-size/CORS layers. |
| `src/cli.rs` | CLI parsing and dispatch into the service. |
| `src/config.rs` | TOML, dotenv, and process-environment configuration. |
| `src/main.rs` | HTTP, stdio, and CLI mode selection. |

## HTTP routes

The HTTP server listens on port `40110` by default:

```text
/mcp       Streamable HTTP MCP transport; authentication depends on AuthPolicy
/health    Public liveness probe
/status    Public, redacted runtime metadata
/.well-known/* and OAuth routes   mounted only in OAuth mode
```

There is no local REST action endpoint or embedded web application. Arcane owns
those surfaces; rarcane provides MCP plus a parity CLI.

## Authentication

- Loopback and stdio use `LoopbackDev`.
- Non-loopback bearer deployments use `Mounted { auth_state: None }`.
- OAuth uses `Mounted { auth_state: Some(_) }`.
- `TrustedGatewayUnscoped` is only for a gateway that already enforces both
  authentication and authorization.

See [AUTH.md](AUTH.md) for the complete deployment contract.
