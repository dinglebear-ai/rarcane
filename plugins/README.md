# plugins

Claude Code and Codex plugin packages for the MCP server. Both platforms share the same skills and MCP connection config — only the manifests differ.

## Structure

```
plugins/rarcane/
├── .claude-plugin/
│   └── plugin.json       # Claude Code manifest
├── .codex-plugin/
│   ├── plugin.json       # Codex manifest
│   └── README.md         # Codex manifest field reference
├── .mcp.json             # Shared MCP server connection config
└── skills/
    ├── rarcane/
    │   └── SKILL.md      # Tool documentation for Claude and Codex
    └── scaffold-project/
        └── SKILL.md      # Turns scaffold_intent JSON into an approval-first plan
```

---

## Manifests

### `.claude-plugin/plugin.json`

Claude Code plugin manifest. Defines the plugin identity, MCP server connection, lifecycle hooks, and user-configurable options.

**User config fields** (set via Claude Code plugin settings):

| Field | Type | Description |
|---|---|---|
| `server_url` | string | MCP HTTP server base URL (default: `http://localhost:3000`) |
| `api_token` | string (sensitive) | Bearer token for auth |
| `no_auth` | boolean | Disable auth (loopback dev only; non-loopback requires an upstream gateway) |
| `auth_mode` | string | `bearer` or `oauth` |
| `public_url` | string | Public URL for OAuth callbacks |
| `google_client_id` | string (sensitive) | Google OAuth client ID |
| `google_client_secret` | string (sensitive) | Google OAuth client secret |
| `auth_admin_email` | string | OAuth admin email |
| `rarcane_api_url` | string | Upstream service URL |
| `rarcane_api_key` | string (sensitive) | Upstream service API key |

**TEMPLATE**: Replace `rarcane_api_url` / `rarcane_api_key` with your service's credential fields.

### `.codex-plugin/plugin.json`

Codex equivalent of the Claude Code manifest. Shares `.mcp.json` and `skills/` with the Claude plugin. Adds Codex-specific UI fields under `interface`:

- `displayName`, `shortDescription`, `longDescription` — registry presentation
- `defaultPrompt` — three sample prompts shown in the Codex UI
- `brandColor` — hex color for the plugin icon (e.g., `#6366F1`)
- `composerIcon`, `logo` — asset paths (512×512 PNG, SVG)

See `.codex-plugin/README.md` for a full field reference and `brandColor` guide.

### `.mcp.json`

Shared MCP server connection config used by both plugins. Points both clients at the same HTTP endpoint with the same auth headers.

```json
{
  "mcpServers": {
    "rarcane": {
      "type": "http",
      "url": "${user_config.server_url}/mcp",
      "headers": {
        "Authorization": "Bearer ${user_config.api_token}"
      }
    }
  }
}
```

---

## Hooks

The plugin ships **no** Claude Code lifecycle hooks. There is no `hooks/`
directory, and no manifest declares a `hooks` key.

Setup is owned by the Rust binary and invoked on demand:

| Command | Purpose |
|---|---|
| `rarcane setup check` | Audit plugin/runtime setup; read-only |
| `rarcane setup repair` | Apply fixes; idempotent |
| `rarcane setup plugin-hook --no-repair` | Contract audit for rollout tooling; no mutation |

The binary reads `CLAUDE_PLUGIN_OPTION_*` env vars from plugin `userConfig`,
exports them as its runtime environment variables, prepares the plugin appdata
directory, and returns a structured JSON report. Deployment policy, repair
behavior, and failure classification live in the binary — it does not manage
Docker, systemd, config rewrites, port conflicts, or OAuth redirect
construction.

---

## Skills

### `skills/rarcane/SKILL.md`

Three-tier structured documentation for the `rarcane` MCP tool, used by both Claude Code and Codex to understand when and how to invoke the tool.

**Tier 1** (above the fold): tool name, quick action table, most common usage.  
**Tier 2**: full action reference — parameters, types, rarcane calls, response shapes.  
**Tier 3**: multi-step workflows demonstrating real-world use.

Also includes HTTP fallback examples using `CLAUDE_PLUGIN_OPTION_SERVER_URL` and `CLAUDE_PLUGIN_OPTION_API_TOKEN` env vars for when the MCP connection isn't available.

**TEMPLATE**: Replace the action table and examples with your service's actual actions.

---

## Version sync

Three files must stay in sync when you bump the version:

| File | Field |
|---|---|
| `Cargo.toml` | `version` |
| `.claude-plugin/plugin.json` | `version` |
| `.codex-plugin/plugin.json` | `version` |

Use `scripts/bump-version.sh patch` (or `minor`/`major`) to update all of them atomically.

---

## TEMPLATE checklist

When adapting this plugin for a real service:

1. Replace all `rarcane` / `Arcane` / `RARCANE_` identifiers with your service name.
2. Update `userConfig` in both `plugin.json` files to match your service's credential fields.
3. Update `skills/rarcane/SKILL.md` with your actual actions, parameters, and examples.
4. Set `brandColor` in `.codex-plugin/plugin.json` to your service's color.
5. Replace `defaultPrompt` entries in the Codex manifest with realistic prompts for your service.
6. Run `scripts/bump-version.sh` after any version change.
