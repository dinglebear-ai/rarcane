# plugins/rarcane — Claude Code instructions

## What this directory is

Multi-platform plugin package for the Arcane MCP server. Contains manifests for Claude Code, Codex, and Gemini CLI — all pointing at the same MCP connection config and skills.

## File map

| File | Role |
|---|---|
| `.claude-plugin/plugin.json` | Claude Code manifest — identity, skills, monitors, `userConfig` |
| `.codex-plugin/plugin.json` | Codex manifest — same data + Codex UI fields (`interface`) |
| `gemini-extension.json` | Gemini CLI manifest — uses `settings` array instead of `userConfig` |
| `.mcp.json` | Shared MCP server connection config used by all three platforms |
| `bin/rarcane` | Release binary used by the monitor — populate with `just install` |
| `monitors/monitors.json` | Background health monitor config (requires Claude Code v2.1.105+) |
| `skills/rarcane/SKILL.md` | Three-tier tool documentation shared by Claude and Codex |

## Versioning rule

**Do not add a `version` field to any manifest.** The marketplace derives version from the git commit SHA. An explicit `version` field causes every push to register as a new version and creates duplicate marketplace entries.

## Updating a manifest

When changing connection config (URL, auth headers), update `.mcp.json` — do not duplicate the values into each manifest separately. All three platforms read `.mcp.json`.

When changing user-configurable settings, update all three manifests: `userConfig` in the Claude and Codex `plugin.json` files, and `settings` in `gemini-extension.json`. Keep field names and descriptions consistent across all three.

## Monitors (Claude Code v2.1.105+)

`monitors/monitors.json` runs `scripts/watch.sh`, which delegates to an installed
`rarcane` on PATH. Plugin monitors must not assume a bundled binary in the
plugin directory.

The monitor command must **not** reference `${user_config.*}`. Claude Code v2.1.207+ rejects it in monitor commands (the command runs through a shell) and the monitor never starts — silently, since a monitor that fails to launch looks the same as one reporting healthy. Monitor processes also receive no `CLAUDE_PLUGIN_OPTION_*`.

`watch.sh` therefore passes no URL; `rarcane watch` resolves its own default from config (`http://localhost:{mcp.port}`). Do not hardcode URLs in `monitors.json` either — if a monitor ever needs a non-default URL, read it from a config file inside the script, which is the documented alternative.

When adding a new monitor: add an entry to `monitors.json` and reference only
scripts under `${CLAUDE_PLUGIN_ROOT}/scripts/`; those scripts should resolve the
runtime binary from PATH and exit non-blocking when it is unavailable.

## Updating the skill

`skills/rarcane/SKILL.md` is shared by Claude Code and Codex. Gemini reads it via the `skills` path in `gemini-extension.json`. Edit it once — all platforms see the change.

The three-tier structure must be preserved:
- **Tier 1** (above fold): tool name, quick action table, critical gotchas
- **Tier 2** (middle): full action reference with parameters and response shapes
- **Tier 3** (bottom): workflows, HTTP fallback, error handling

## Lifecycle hooks — retired

This plugin ships **no** Claude Code lifecycle hooks. There is no `hooks/`
directory, and neither `.claude-plugin/plugin.json` nor `gemini-extension.json`
declares a `hooks` key. `scripts/validate-plugin-layout.sh`,
`cargo xtask patterns`, and `tests/plugin_contract.rs` all fail if hooks are
reintroduced.

## Updating setup

Config reaches the server through the `env` block in `.mcp.json`, which maps
`userConfig` keys to `RARCANE_*` via `${user_config.*}`. That is the live path —
no hook, no manual step.

`apply_plugin_options()` in `src/cli/setup.rs` is a parallel
`CLAUDE_PLUGIN_OPTION_*` → `RARCANE_*` map that only runs under a manual
`rarcane setup plugin-hook`. **When you add or rename a `userConfig` field,
update both the `.mcp.json` env block and that mapping** — updating only
`setup.rs` leaves the value unreachable in a real plugin install.

Setup is otherwise owned by the binary and invoked on demand (`rarcane setup
check`, `rarcane setup repair`, `rarcane setup plugin-hook --no-repair`) for
appdata and preflight.

Caveat: only Claude Code and Codex read `.mcp.json`. Gemini declares its own
HTTP `mcpServers` block inside `gemini-extension.json` and must be updated
separately.

Sensitive fields declared `"sensitive": true` in `plugin.json` are exposed as
env vars to the binary but are **never** substituted into skill content.

## Template adaptation

When renaming `rarcane` → your service:

1. Replace all `rarcane` / `Arcane` / `RARCANE_` identifiers in every file in this directory.
2. Rename `skills/rarcane/` to `skills/<your-service>/`.
3. Update `apply_plugin_options()` in `src/cli/setup.rs` — it maps `CLAUDE_PLUGIN_OPTION_*` to your service's actual `RARCANE_*` vars.
4. Keep the no-version rule: do not add `"version"` to any manifest.
5. Keep the no-hooks rule: do not add a `hooks` key or a `hooks/` directory.
