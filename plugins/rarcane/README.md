# rarcane plugin

Multi-platform plugin package that connects Claude Code, Codex, and Gemini CLI to the Arcane MCP server.

## Structure

```
plugins/rarcane/
├── .claude-plugin/
│   └── plugin.json         # Claude Code manifest
├── .codex-plugin/
│   ├── plugin.json         # Codex manifest
│   └── README.md           # Codex manifest field reference
├── gemini-extension.json   # Gemini CLI extension manifest
├── .mcp.json               # Shared MCP server connection config (all three platforms)
├── bin/
│   └── rarcane             # Release binary (populate with: just install)
├── monitors/
│   └── monitors.json       # Background health monitor (requires Claude Code v2.1.105+)
└── skills/
    └── rarcane/
        └── SKILL.md        # Tool documentation (shared by Claude and Codex)
```

## Platform manifests

Claude Code and Codex read their MCP connection config from the shared `.mcp.json`. Gemini CLI embeds its `mcpServers` config inline in `gemini-extension.json` (its own format). All three share the same `skills/` directory.

| File | Platform | MCP config | Variable syntax |
|---|---|---|---|
| `.claude-plugin/plugin.json` | Claude Code | `.mcp.json` | `${user_config.*}` |
| `.codex-plugin/plugin.json` | Codex | `.mcp.json` | `${user_config.*}` |
| `gemini-extension.json` | Gemini CLI | inline `mcpServers` | `${settings.*}` |

**No `version` field in any manifest.** The marketplace assigns version from the git commit SHA. Adding an explicit version creates duplicate entries on every push.

## MCP connection

`.mcp.json` is shared across all platforms:

```json
{
  "mcpServers": {
    "rarcane": {
      "type": "http",
      "url": "${user_config.server_url}/mcp",
      "headers": { "Authorization": "Bearer ${user_config.api_token}" }
    }
  }
}
```

The `${user_config.*}` / `${settings.*}` variables are populated from each platform's user-configurable settings at runtime.

## Hooks

This plugin ships **no** Claude Code lifecycle hooks — no `hooks/` directory and
no `hooks` key in any manifest.

Setup is owned by the binary and run on demand instead of at session start:

```bash
rarcane setup check                     # audit plugin/runtime setup
rarcane setup repair                    # apply fixes
rarcane setup plugin-hook --no-repair   # contract audit, no mutation
```

The binary maps `CLAUDE_PLUGIN_OPTION_*` settings to environment variables,
prepares appdata, and returns a structured JSON report.

## Monitors

**Requires Claude Code v2.1.105+.**

`monitors/monitors.json` declares a background `server-health` monitor that starts automatically at session start. It runs `scripts/watch.sh`, which delegates to an installed `rarcane` on PATH, and delivers each stdout line to Claude as a notification whenever the MCP server changes state.

The monitor emits only on state transitions — Claude is not notified while the server is stable. Three states:

- `UP` — `/health` returned 2xx
- `DOWN` — connection refused / timeout
- `DEGRADED(HTTP N)` — non-2xx HTTP response

The plugin does not ship or install a binary. Install `rarcane` separately before
enabling the monitor.

Disabling the plugin mid-session does not stop an already-running monitor; it stops when the session ends.

## Skills

`skills/rarcane/SKILL.md` is the three-tier structured documentation for the `rarcane` MCP tool. The AI reads Tier 1 for quick lookups, Tier 2 for parameter details, Tier 3 for multi-step workflows.

## Packaging checklist

1. Confirm the plugin does not rely on a bundled `rarcane` binary.
2. Confirm `rarcane` is installed separately when testing runtime setup.
3. Run `cargo test --test plugin_contract`.
4. Verify all manifests still omit explicit `version` fields.
5. Install through the target marketplace or local plugin path.
