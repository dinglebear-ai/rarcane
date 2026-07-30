---
title: "MCP Schema Contract"
created: "2026-07-30"
updated: "2026-07-30"
---

# MCP Schema Contract

Generated from `src/actions.rs` and checked against the schema, README, skill docs, help text, and scope routing.

Run:

```bash
python3 scripts/check-schema-docs.py --write
python3 scripts/check-schema-docs.py --check
```

## Tool

| Field | Value |
|---|---|
| Tool name | `rarcane` |
| Schema resource | `rarcane://schema/mcp-tool` |
| Dispatch parameter | `action` |

## Actions

| Action | Scope | Description |
|---|---|---|
| `help` | public | Return the in-tool action reference. Public; no scope required. |
| `status` | `rarcane:read` | Return local server status without contacting Arcane. |
| `elicit_name` | `rarcane:read` | Ask the MCP client for a name and return a greeting. |
| `scaffold_intent` | `rarcane:read` | Collect scaffold requirements and return a side-effect-free handoff contract. |
| `environment` | `rarcane:read` / `rarcane:write` | List, inspect, test, create, update, and delete Arcane environments. |
| `project` | `rarcane:read` / `rarcane:write` | List, inspect, deploy, build, and manage Arcane projects. |
| `container` | `rarcane:read` / `rarcane:write` | List, inspect, start, stop, restart, update, and remove containers. |
| `image` | `rarcane:read` / `rarcane:write` | List, inspect, pull, prune, scan, and remove images. |
| `network` | `rarcane:read` / `rarcane:write` | List, inspect, create, prune, and remove networks. |
| `volume` | `rarcane:read` / `rarcane:write` | List, inspect, browse, back up, restore, create, prune, and remove volumes. |
| `system` | `rarcane:read` / `rarcane:write` | Retrieve system information and run supported system operations. |
| `image-update` | `rarcane:read` | Check image update status and summaries. |
| `vulnerability` | `rarcane:read` / `rarcane:write` | Inspect, ignore, and unignore vulnerability findings. |
| `registry` | `rarcane:read` / `rarcane:write` | List, inspect, test, create, update, and delete registries. |
| `gitops` | `rarcane:read` / `rarcane:write` | List, inspect, browse, sync, create, update, and delete GitOps entries. |

## Drift Rules

- `ACTION_SPECS` in `src/actions.rs` is the canonical action and scope list.
- `src/mcp/schemas.rs` must derive its enum from `ACTION_SPECS`.
- The MCP tool schema must reject unknown top-level parameters and encode action-specific requirements that fit the single-tool dispatch model.
- `help` is intentionally public and must have no required scope.
- `src/mcp/tools.rs`, `README.md`, and `plugins/rarcane/skills/rarcane/SKILL.md` must mention every action.
- `src/mcp/rmcp_server.rs` owns stable resources and must keep `rarcane://schema/mcp-tool` wired to `tool_definitions()`.
- `src/mcp/prompts.rs` owns stable prompts and must keep `quick_start` covered by prompt tests.

## Resources

| URI | Source | Contract |
|---|---|---|
| `rarcane://schema/mcp-tool` | `src/mcp/rmcp_server.rs` | Returns `tool_definitions()` as `application/json`. |

## Prompts

| Prompt | Source | Contract |
|---|---|---|
| `quick_start` | `src/mcp/prompts.rs` | Guides a client to call `status` and public `help`. |

## Input Validation

- `action` is always required.
- `elicit_name` and `scaffold_intent` collect their extra fields through MCP elicitation, not direct tool-call arguments.
- Unknown top-level parameters are rejected by the schema.
