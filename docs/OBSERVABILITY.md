---
title: "Observability"
created: "2026-07-30"
updated: "2026-07-30"
---

# Observability

## HTTP probes

| Endpoint | Auth | Current contract |
|---|---|---|
| `GET /health` | Public | Liveness only: `{"status":"ok"}`. It does not contact Arcane. |
| `GET /status` | Public | Redacted server name, package version, and `transport: "http"`. |
| `/mcp` | Auth policy | Streamable HTTP MCP endpoint. |

Use `rarcane doctor` for an active upstream-connectivity check. Do not use
`/health` as an Arcane readiness signal.

## Logging

The running binary currently initializes `tracing_subscriber` on stderr. Under
systemd, read these logs from the journal; under Docker, use the container log
driver. `RUST_LOG` controls filtering, for example:

```bash
RUST_LOG=info,rmcp=warn rarcane serve
journalctl --user -u rarcane-mcp.service -f
docker logs -f arcane-rmcp
```

The repository contains reusable file-formatting helpers in `src/logging.rs`,
but startup does not wire them in and no `~/.rarcane/logs/rarcane.log` file is
promised by the current runtime.

MCP calls log start, completion, elapsed time, and a redacted execution failure.
The MCP client receives stable validation errors and a generic internal error;
inspect stderr/journal/container logs for operational detail.
