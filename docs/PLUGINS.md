# Plugin Surfaces

This template ships one service plugin package with three host-specific entrypoints:

- Claude Code: `plugins/rarcane/.claude-plugin/plugin.json`
- Codex: `plugins/rarcane/.codex-plugin/plugin.json`
- Gemini: `plugins/rarcane/gemini-extension.json`

All three surfaces should describe the same MCP server, expose the same skills, and connect to the same HTTP MCP endpoint. The host manifests differ, but the service behavior should not.

## Layout

```text
plugins/rarcane/
  .claude-plugin/
    plugin.json          # Claude Code manifest
  .codex-plugin/
    plugin.json          # Codex manifest
    README.md            # Codex manifest field reference
  .mcp.json              # Shared Claude/Codex MCP connection config
  gemini-extension.json  # Gemini CLI extension manifest
  bin/
    rarcane              # Plugin binary; owns setup and repair behavior
  skills/
    rarcane/
      SKILL.md           # Shared action documentation
    scaffold-project/
      SKILL.md           # Approval-first template adaptation handoff skill
```

When adapting the template, rename `rarcane`, `Arcane`, and `EXAMPLE` consistently across the package, then update host-specific display text and credentials.

## Shared Contract

Each plugin surface should agree on:

- service name and repository URL
- MCP server name
- HTTP MCP URL shape: `<server_url>/mcp`
- bearer token setting name
- upstream service credential names
- action list and skill documentation
- read/write capability claims

Keep the plugin manifests thin. Runtime setup belongs in the service binary, not in manifest-specific shell code.

## Claude Code

Claude Code uses `plugins/rarcane/.claude-plugin/plugin.json`.

Responsibilities:

- identifies the plugin and repository
- points at the shared `.mcp.json` and `skills` paths
- defines `userConfig` settings exposed in Claude Code
- marks sensitive values with `sensitive: true`

### No lifecycle hooks

This plugin ships **no** Claude Code lifecycle hooks. `plugins/rarcane/hooks/`
was retired; neither the Claude manifest nor the Gemini manifest declares a
`hooks` key, and `scripts/validate-plugin-layout.sh` plus `cargo xtask patterns`
both fail if hooks are reintroduced.

Setup still lives in the binary and is run on demand rather than on session
start. The family-standard command remains:

```bash
<binary> setup plugin-hook
```

For rollout audits, the binary must also support:

```bash
<binary> setup plugin-hook --no-repair
```

Concretely, for this server:

```bash
rarcane setup check     # audit current plugin/runtime setup
rarcane setup repair    # apply fixes
rarcane setup plugin-hook --no-repair   # contract audit for rollout tooling
```

The binary maps `CLAUDE_PLUGIN_OPTION_*` values into runtime env vars, audits
the appdata setup, and emits a JSON report carrying `exit_policy`,
`blocking_failures`, `advisory_failures`, and `ran_repair`. Setup must not own
Docker/systemd orchestration or service bootstrap behavior.

## Codex

Codex uses `plugins/rarcane/.codex-plugin/plugin.json`.

Responsibilities:

- identifies the plugin for Codex listings
- points at shared `skills` and `.mcp.json`
- describes the interface shown in Codex UI
- declares read/write capabilities
- provides rarcane prompts
- provides branding fields such as `brandColor`, `composerIcon`, and `logo`

Codex does not use Claude lifecycle hooks. Its manifest should still point to the same MCP server and shared skills so behavior stays aligned with Claude Code.

Codex-specific fields to adapt:

| Field | Purpose |
| --- | --- |
| `interface.displayName` | human-readable plugin name |
| `interface.shortDescription` | short listing text |
| `interface.longDescription` | full listing text |
| `interface.capabilities` | `["Read"]` or `["Read", "Write"]` |
| `interface.defaultPrompt` | three realistic prompts |
| `interface.brandColor` | service-appropriate hex color |

See `plugins/rarcane/.codex-plugin/README.md` for the full manifest field reference.

## Gemini

Gemini uses `plugins/rarcane/gemini-extension.json`.

Responsibilities:

- identifies the extension
- declares Gemini settings
- connects to the MCP HTTP endpoint
- points at shared skills
- optionally points Gemini at a context file with `contextFileName`

The Gemini manifest uses `settings.*` interpolation instead of Claude/Codex `user_config.*` interpolation:

```json
"url": "${settings.server_url}/mcp"
```

Sensitive Gemini settings use:

```json
"secret": true
```

Keep Gemini setting names aligned with Claude/Codex where possible. For rarcane, prefer `server_url`, `api_token`, `<service>_api_url`, and `<service>_api_key` across all three surfaces.

## Plugin Validation

Run the plugin layout validator after changing manifests, MCP config, or
skills:

```bash
just validate-plugin
# or
scripts/validate-plugin-layout.sh
```

The validator checks:

- Claude, Codex, and Gemini manifests are valid JSON
- plugin manifests do not contain a `version` field
- manifests point to the shared `.mcp.json` and skills paths
- shared MCP config exposes the `rarcane` HTTP server at `${user_config.server_url}/mcp`
- Gemini config exposes the same `rarcane` HTTP server at `${settings.server_url}/mcp`
- no manifest declares a `hooks` key and no `hooks/` directory is shipped
- every skill has `name:` and `description:` frontmatter

Use `PLUGIN_ROOT=plugins/<service>` when validating an adapted service package.

For release checks, `just pre-release` includes this validator and the other
template gates.

## Shared MCP Config

Claude Code and Codex share `plugins/rarcane/.mcp.json`:

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

Gemini carries equivalent MCP config directly in `gemini-extension.json` because its interpolation model is different.

## Skills

`plugins/rarcane/skills/rarcane/SKILL.md` is shared across Claude, Codex, and Gemini. Every skill follows the three-tier fallback pattern — agents try each tier in order and stop when one works:

The shared service skill also documents the MCP-only `elicit_name` and
`scaffold_intent` workflows. `scaffold_intent` returns a side-effect-free
planning contract for the `scaffold-project` skill; it never grants permission
to mutate files. Because both actions require an MCP peer, they do not have CLI
fallback commands.

```markdown
# rarcane — Claude Code Skill

Use this skill whenever you need to query or manage the Arcane service.

## Tier 1: MCP tool (preferred)
Use when the rarcane MCP server is configured in your agent.

rarcane(action="things")
rarcane(action="thing", id="abc123")
rarcane(action="help")          # always available, no auth required

## Tier 2: CLI binary
Use when MCP is unavailable but the binary is installed in $PATH.

rarcane things [--json]
rarcane thing <id> [--json]
rarcane status

Env required: RARCANE_API_URL, RARCANE_API_KEY

## Tier 3: Direct API (last resort)
Use when neither MCP nor CLI is available.

curl -H "Authorization: Bearer $RARCANE_API_KEY" \
     "$RARCANE_API_URL/things"

## Gotchas
- [service-specific pitfalls go here]
- [e.g. pagination, required headers, rate limits]
```

The skill should also include:

- quick action table (action → description → required params)
- full parameter reference with types
- common workflows (status check → list → inspect)
- response shapes for key actions
- sensitive-value handling notes (never log tokens, etc.)

Do not maintain separate skill docs per host. Update the shared skill when the action surface changes; Claude, Codex, and Gemini all read the same file.

## Binary-Owned Hook Standard

Every Rust server with a Claude plugin should expose:

```bash
<binary> setup plugin-hook
<binary> setup plugin-hook --no-repair
<binary> setup check
<binary> setup repair
```

`setup plugin-hook` should:

- run `setup check` first
- run `setup repair` only when needed and only when `--no-repair` is absent
- emit structured JSON when the global JSON flag is used
- include `exit_policy`, `blocking_failures`, `advisory_failures`, `ran_repair`, and `no_repair`
- exit `0` for success or advisory failures
- exit nonzero for blocking failures
- enforce a bounded total hook runtime

Advisory failures are non-blocking local conditions such as missing `.env` files when process env already supplies values, occupied MCP ports, optional startup proofs, or model prewarm. Blocking failures are prerequisites required for the plugin to function, such as missing appdata directories, missing required upstream credentials, or invalid OAuth/auth configuration.

Use `scripts/check-plugin-hook-contract.py` to audit the cross-repo standard:

```bash
# Static hook/delegation checks for all known Rust servers.
scripts/check-plugin-hook-contract.py

# Also run each binary's `setup plugin-hook --no-repair` JSON contract.
scripts/check-plugin-hook-contract.py --execute
```

## Version And Release Sync

Keep version and metadata synchronized across:

| File | Fields |
| --- | --- |
| `Cargo.toml` | package `version`, homepage/repository when present |
| `plugins/rarcane/.claude-plugin/plugin.json` | identity, repository, user config; no `version` field |
| `plugins/rarcane/.codex-plugin/plugin.json` | identity, repository, interface metadata; no `version` field |
| `plugins/rarcane/gemini-extension.json` | identity, repository, settings |
| `server.json` | package version and registry metadata, when present |

`Cargo.toml` is the canonical version source for this template. Use
`scripts/bump-version.sh` to update Cargo and `server.json` together, then use
`scripts/check-version-sync.sh` or `just pre-release` to verify that
version-bearing files still agree. Plugin manifests should remain versionless.

The template should not claim write capability unless the MCP server has real write actions. Read-only servers should use Codex `["Read"]` and avoid write-oriented sample prompts.

## Adaptation Checklist

When creating a real server from the template:

1. Rename `rarcane`, `Arcane`, and `EXAMPLE` across plugin files.
2. Update all three manifests with the real repository, description, author, keywords, and capability claims.
3. Keep credential names aligned across Claude `userConfig`, Codex shared `.mcp.json`, and Gemini `settings`.
4. Replace upstream credential fields such as `rarcane_api_url` and `rarcane_api_key`.
5. Update `apply_plugin_options()` in `src/cli/setup.rs` to map service-specific plugin options into env vars.
6. Implement `<binary> setup plugin-hook`, `--no-repair`, `check`, and `repair`.
7. Update shared skill docs for the actual action surface.
8. Replace Codex `defaultPrompt` entries with realistic prompts.
9. Update Gemini `description`, `settings`, and `contextFileName` if needed.
10. Run `just validate-plugin`, plugin contract tests, and `scripts/check-plugin-hook-contract.py` before release.

## Required Tests

Each server should include tests that prove:

- no plugin manifest declares a `hooks` key and no `hooks/` directory exists
- `setup plugin-hook --no-repair` parses and does not mutate appdata
- JSON plugin-hook output contains `exit_policy`, `blocking_failures`, `advisory_failures`, `ran_repair`, and `no_repair`
- advisory failures exit `0`
- blocking failures exit nonzero
- Claude, Codex, and Gemini manifests use the same service name, endpoint, token setting, and credential fields
