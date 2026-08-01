# rarcane — Claude Code instructions

## What this project is

A Rust MCP server and CLI that bridges agents to an [Arcane](https://github.com/ofkm/arcane)
API server for Docker management — environments, compose projects, containers,
images, networks, volumes, registries, GitOps repos, image-update checks,
vulnerability scanning, and system operations.

| Fact | Value |
|---|---|
| Repository | `git@github.com:dinglebear-ai/rarcane.git` (default branch `main`) |
| Cargo workspace | 2 members: `.` (the `rarcane` crate) and `xtask` |
| Edition / MSRV | 2024 / Rust 1.97.1 |
| MCP runtime | `rmcp` 3.0.0-beta.2 |
| Binary / CLI | `rarcane` |
| npm package | `@dinglebear/rarcane` (`packages/arcane-rmcp/`) |
| MCP tool name | `arcane` (one action-dispatched tool) |
| Default HTTP port | `40110` |
| Config home | `~/.rarcane` on hosts, `/data` in containers |

The repo doubles as the reference implementation for the rmcp server family,
while its root manifest and runtime configuration describe the live rarcane
service rather than scaffold placeholders.

## Module map

| File | Role |
|------|------|
| `src/arcane.rs` | `ArcaneClient` — authenticated Arcane HTTP transport |
| `src/app.rs` | `ArcaneService` — business layer; all logic lives here, never in shims |
| `src/actions.rs` | `ACTION_SPECS` — the action/subaction table (81 specs), scopes, destructive flags, validation errors |
| `src/server.rs` | `AppState`, `AuthPolicy`, `build_auth_layer` — HTTP server state and auth policy |
| `src/server/routes.rs` | Axum router: `/mcp`, `/health`, `/status`, OAuth discovery routes |
| `src/mcp.rs` | MCP protocol layer — re-exports from `mcp/` submodules |
| `src/mcp/tools.rs` | MCP shim: parse JSON args → call service → return `Value` |
| `src/mcp/schemas.rs` | Tool JSON schema derived from `ACTION_SPECS` |
| `src/mcp/rmcp_server.rs` | `ServerHandler` impl: tools, resources, prompts, scope checks |
| `src/mcp/prompts.rs` | MCP prompts (`quick_start`) |
| `src/mcp/transport.rs` | Streamable HTTP transport wiring and session lifecycle |
| `src/config.rs` | `Config`, `ArcaneConfig`, `McpConfig`, `AuthConfig`, env loading |
| `src/cli.rs` | CLI shim: parse args → call service → print |
| `src/cli/doctor.rs` + `src/cli/doctor/checks.rs` | Pre-flight checks: env, connectivity, config validation |
| `src/cli/setup.rs` | Setup/repair, `apply_plugin_options()`, `install_self()` |
| `src/cli/watch.rs` | Polls `/health` and emits state-change lines for the plugin monitor |
| `src/logging.rs` + `src/logging/{aurora,formatter}.rs` | Aurora-themed tracing output |
| `src/token_limit.rs` | Token budget enforcement for MCP response payloads |
| `src/main.rs` | Mode dispatch: HTTP server / stdio / CLI |
| `src/lib.rs` | Public API + `testing` helpers for integration tests |
| `xtask/` | Repo automation: `dist`, `ci`, `patterns`, `symlink-docs`, `check-env`, `check-test-siblings` |

### Test layout

Unit tests live in **sibling files**, never inline `#[cfg(test)] mod tests` and
never `tests/mod.rs`: `src/app.rs` pairs with `src/app_tests.rs`. `cargo xtask
check-test-siblings` enforces the pairing, and `mod_module_files = "deny"` is
the module-layout rule (`foo.rs` next to `foo/`, never `foo/mod.rs`).

Integration tests:

| File | Covers |
|------|--------|
| `tests/cli_parse.rs` | CLI argument parsing |
| `tests/tool_dispatch.rs` | MCP tool dispatch (service layer, no real credentials) |
| `tests/plugin_contract.rs` | Plugin manifest identity, versionless rule, no-hooks rule, setup contract |
| `tests/template_invariants.rs` | Repo-shape invariants |
| `tests/mcporter/` | mcporter-driven MCP integration checks (requires a running server) |

## The thin-shim rule — enforce this hard

`src/mcp/tools.rs` and `src/cli.rs` contain **zero business logic**. They only:
1. Parse their input format (JSON args or CLI flags)
2. Call the corresponding `ArcaneService` method
3. Return the result

If you find yourself computing, filtering, transforming, or validating data in `tools.rs` or `cli.rs`, stop and move it to `app.rs`.

## How to add an action

Most Arcane domains are pure table entries: adding a subaction to an existing
domain is usually **only** step 3 plus tests, because `ACTION_SPECS` drives the
schema, the help text, and the generic `call` dispatch.

1. **`src/arcane.rs`** — add transport behavior only when an action needs a distinct upstream request path.

2. **`src/app.rs`** — add a delegating method when the action is not covered by the generic `dispatch()` path.

3. **`src/actions.rs`** — add the action to `ACTION_SPECS` via the `spec!` macro: action, subaction, HTTP method, upstream path, required scope, `env`, `id`, `destructive`, `body`.

4. **`src/mcp/schemas.rs`** — add any *new parameter names* to `tool_definitions()`; the action enum itself is derived from `ACTION_SPECS`.

5. **`src/mcp/tools.rs`** — add a match arm only for non-generic actions. Update `HELP_TEXT`.

6. **`src/cli.rs`** — add a `Command` variant, a parse arm in `parse_args()`, and a dispatch arm in `run()` for non-generic actions.

7. **Tests** — add coverage in `tests/tool_dispatch.rs` and the relevant `*_tests.rs` sibling.

8. **`docs/MCP_SCHEMA.md`** — regenerate with `python3 scripts/check-schema-docs.py` (CI runs it with `--check` and fails on drift).

9. **`CHANGELOG.md`** — add an entry under `[Unreleased]`.

For actions with parameters, extract them with `string_arg(&args, "param_name")` in `tools.rs`.

## Action surface

One MCP tool, `arcane`, dispatched by `action` + (for most domains) `subaction`.
`src/actions.rs` is the source of truth; `docs/MCP_SCHEMA.md` is its generated
contract.

| Action | Subactions | Notes |
|---|---|---|
| `help` | — | Public; no scope required |
| `status` | — | Local bridge + Arcane config status |
| `elicit_name` | — | MCP-only elicitation demo |
| `scaffold_intent` | — | MCP-only elicitation + skill handoff |
| `environment` | `list get create update delete test` | |
| `project` | `list get create update up down restart pull destroy redeploy build` | `down`/`restart`/`destroy`/`redeploy` are destructive |
| `container` | `list get create start stop restart update delete stats` | |
| `image` | `list get pull delete prune scan` | |
| `network` | `list get create delete prune` | |
| `volume` | `list get create delete prune browse list-backups create-backup delete-backup restore restore-files` | |
| `system` | `prune start-all stop-all docker-info convert` | |
| `image-update` | `check-all check check-batch summary` | read-only |
| `vulnerability` | `summary list scanner-status ignore unignore list-ignored` | |
| `registry` | `list get create update delete test` | |
| `gitops` | `list get create update delete sync status browse` | |

Destructive subactions are rejected unless `params.confirm=true` (MCP) or
`--confirm` (CLI). `RARCANE_MCP_ALLOW_DESTRUCTIVE=true` relaxes that gate
globally — do not set it in shared deployments.

## Surface parity policy

rarcane is an **upstream-client MCP server**: the required surfaces are MCP +
CLI. Do not add a REST API or web UI — Arcane already owns those. Every business
action must be reachable from both shims, which is automatic because both call
the same `ArcaneService`.

## Auth model

`AuthPolicy` is an enum with three states:

| Variant | When | Effect |
|---------|------|--------|
| `AuthPolicy::LoopbackDev` | `no_auth=true` or host is loopback (`localhost`, `127.*`, `::1`) via `McpConfig::is_loopback()` | No auth middleware; scope checks bypassed |
| `AuthPolicy::TrustedGatewayUnscoped` | `RARCANE_NOAUTH=true` on non-loopback behind an authz-enforcing gateway | No auth middleware; scope checks bypassed |
| `AuthPolicy::Mounted { auth_state: None }` | Default non-loopback | Static bearer token required |
| `AuthPolicy::Mounted { auth_state: Some(_) }` | `auth_mode = "oauth"` | Full Google OAuth + RS256 JWT issuance |

Auth is selected in `build_auth_policy()` in `main.rs`. Scopes are `rarcane:read` and `rarcane:write` (write satisfies read). `help` requires no scope. Unknown actions get `DENY_SCOPE`.

## Environment variables

`src/config.rs` is authoritative. If a doc disagrees with it, the doc is wrong.

| Variable | Default | Description |
|----------|---------|-------------|
| `RARCANE_API_URL` | — | Arcane API base URL |
| `RARCANE_API_KEY` | — | Arcane API key / bearer token |
| `RARCANE_MCP_HOST` | `127.0.0.1` | Bind host (loopback by default, deliberately) |
| `RARCANE_MCP_PORT` | `40110` | Bind port |
| `RARCANE_MCP_SERVER_NAME` | `arcane-rmcp` | MCP server name advertised to clients |
| `RARCANE_MCP_NO_AUTH` | `false` | Disable auth (loopback only) |
| `RARCANE_NOAUTH` | `false` | Trust an upstream gateway to enforce auth (non-loopback) |
| `RARCANE_MCP_TOKEN` | — | Static bearer token |
| `RARCANE_MCP_ALLOWED_HOSTS` | — | Extra comma-separated Host header values |
| `RARCANE_MCP_ALLOWED_ORIGINS` | — | Extra comma-separated CORS origins |
| `RARCANE_MCP_PUBLIC_URL` | — | Public URL for OAuth metadata endpoints |
| `RARCANE_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth` (invalid values fail startup) |
| `RARCANE_MCP_GOOGLE_CLIENT_ID` | — | Google OAuth client ID |
| `RARCANE_MCP_GOOGLE_CLIENT_SECRET` | — | Google OAuth client secret |
| `RARCANE_MCP_AUTH_ADMIN_EMAIL` | — | OAuth admin email allowlist bootstrap |
| `RARCANE_MCP_ALLOW_DESTRUCTIVE` | `false` | Bypass the per-call destructive confirmation gate |
| `RARCANE_HOME` | `~/.rarcane` | Config/appdata root override |
| `RARCANE_DOCTOR_ACCEPT_INVALID_CERTS` | `false` | Let `doctor` skip TLS verification |
| `RUST_LOG` | `info` | Log filter |

## Elicitation

The `elicit_name` action demonstrates MCP elicitation (spec 2025-06-18). The server calls `peer.elicit::<T>()` to ask the MCP client for user input mid-call. The type `T` must:
- Derive `JsonSchema`, `Serialize`, `Deserialize`
- Be an object (struct), not a primitive
- Be registered with `rmcp::elicit_safe!(T)`

`ElicitationError::CapabilityNotSupported` is handled gracefully — clients that don't support it get a fallback message instead of an error.

## Build commands

```bash
cargo build --release     # produces target/release/rarcane
cargo test                # all tests
cargo clippy -- -D warnings  # lint (must pass)
cargo fmt                 # format

just dev                  # RARCANE_MCP_HOST=127.0.0.1 RARCANE_MCP_NO_AUTH=true cargo run -- serve mcp (loopback only, no auth)
just check                # cargo check
just test                 # cargo test
just lint                 # cargo clippy -- -D warnings
just fmt                  # cargo fmt
just verify               # fmt-check → lint → check → test
just gen-token            # openssl rand -hex 32
just health               # curl http://localhost:40110/health | jq .

just template-check       # patterns + plugin layout + schema docs + scaffold contract + feature smoke
just validate-plugin      # scripts/validate-plugin-layout.sh
just ci                   # cargo xtask ci — full CI suite
```

## Test helpers

`src/lib.rs` exports `testing::loopback_state()` and `testing::bearer_state(token)` (behind `features = ["test-support"]` or `cfg(test)`). Use these in integration tests — they build `AppState` without real credentials.

## CLI ↔ MCP action parity

Every action in the MCP tool must also be reachable from the CLI, and vice versa.
Both shims call the same `ArcaneService` methods, so parity is automatic when the
shims are complete.

**Exception — MCP-only features:** `elicit_name` and MCP resources/prompts have no
CLI equivalent. Elicitation requires a live MCP client interaction (the server asks
the user for input mid-call via `peer.elicit()`); that interaction model does not
translate to a one-shot CLI call. Resources and prompts are MCP protocol concepts
with no CLI analogue.

The MCP tool is named `arcane`; the CLI binary is named `rarcane`.

| Service Method | MCP Action | CLI Command | Notes |
|---|---|---|---|
| `service.status()` | `arcane(action="status")` | `rarcane status` | |
| `service.dispatch(action)` | `arcane(action="container", subaction="list", ...)` | `rarcane call --action container --subaction list ...` | Generic Arcane action parity — covers every domain in the action table |
| _(MCP client interaction)_ | `arcane(action="elicit_name")` | _(MCP-only — no CLI equivalent)_ | Requires elicitation-capable client |
| _(MCP elicitation wizard)_ | `arcane(action="scaffold_intent")` | _(MCP-only — no CLI equivalent)_ | Combines elicitation + skill handoff; no one-shot CLI equivalent |
| _(built-in)_ | `arcane(action="help")` | `rarcane help --domain <domain>` / `rarcane --help` | MCP returns structured JSON; CLI prints usage |

`watch`, `serve`, `mcp`, `doctor`, and `setup` are CLI infrastructure, not MCP
actions, and are exempt from parity.

## MCP features implemented

- **Tools** — one `arcane` tool with action dispatch
- **Resources** — `rarcane://schema/mcp-tool` (JSON schema for the tool)
- **Prompts** — `quick_start`
- **Elicitation** — `elicit_name` and `scaffold_intent` use `peer.elicit()` (spec 2025-06-18)
- **Scaffold handoff** — `scaffold_intent` returns JSON only; the `scaffold-project` plugin skill turns it into an approval-first plan

## Plugin packaging

`plugins/rarcane/` ships three host manifests (`.claude-plugin/plugin.json`,
`.codex-plugin/plugin.json`, `gemini-extension.json`) over one shared
`.mcp.json` and one shared `skills/` directory.

**No `version` field in any plugin manifest.** The marketplace derives the
version from the git commit SHA on every push — an explicit version makes every
push register as a new version and creates duplicate entries. Do not add
`version`, and do not point `scripts/bump-version.sh` at plugin manifests.

**No Claude Code lifecycle hooks.** `plugins/rarcane/hooks/` was retired; no
manifest declares a `hooks` key. `scripts/validate-plugin-layout.sh`,
`cargo xtask patterns`, and `tests/plugin_contract.rs` all fail if hooks are
reintroduced.

**How plugin config reaches the runtime.** Through the `env` block in
`plugins/rarcane/.mcp.json`, which maps 10 `userConfig` keys to `RARCANE_*`
(`${user_config.rarcane_api_url}` → `RARCANE_API_URL`, and so on). This is the
live path and needs no hook and no manual step.

`apply_plugin_options()` in `src/cli/setup.rs` is a *second*, parallel map from
`CLAUDE_PLUGIN_OPTION_*` → `RARCANE_*`. It only runs when the parsed command is
`Setup(SetupCommand::PluginHook)` (`src/main.rs:130-135`), so with hooks gone it
is reachable only via a manual `rarcane setup plugin-hook`. **Adding or renaming
a `userConfig` key means updating both maps.** Setup subcommands (`check`,
`repair`, `plugin-hook --no-repair`) remain useful for appdata and preflight,
not for configuring the server.

`server_url` is deliberately **not** in the `.mcp.json` env block: nothing in
`src/` ever read `RARCANE_SERVER_URL`, and the monitor no longer passes a URL
either. It remains a `userConfig` field because Gemini's HTTP `mcpServers` block
consumes it.

## Common gotchas

- **Stdio mode suppresses logs** — `main.rs` sets log level to `warn` in stdio mode so JSON-RPC is not corrupted by log lines on stdout.
- **The MCP tool is `arcane`, the binary is `rarcane`** — they intentionally differ. Client config registers the server as `rarcane`, but `tools/call` uses `name: "arcane"`.
- **`config.toml` / `config.example.toml` still carry `example` / `EXAMPLE` template placeholders** — they are scaffolding for derived servers, not live rarcane config. Real settings come from env and `~/.rarcane/`.
- **Scope checks run in `rmcp_server.rs`**, not in `tools.rs`. `tools.rs` only dispatches.
- **`help` action is public** — `required_scope_for("help")` returns `None`. All other actions require at least `rarcane:read`; `rarcane:write` satisfies read. Unknown actions get `DENY_SCOPE`.
- **Default bind is `127.0.0.1:40110`** — `default_mcp_host()` / `default_mcp_port()` in `config.rs`. Binding non-loopback without a token, OAuth, or `RARCANE_NOAUTH` is a misconfiguration.
- **`elicit_name` and `scaffold_intent` are MCP-only** — elicitation requires a live client connection. These are the two intentional parity exceptions.
- **`watch`, `serve`, `mcp`, `doctor`, and `setup` are CLI infrastructure** — not MCP actions, no parity requirement. `watch` polls `/health` and emits state-change lines to stdout (used by the plugin monitor).
- **`lab-auth` is a pinned git dependency** on `dinglebear-ai/labby` at rev `87cec32`. Do not bump it casually — 9 repos in the family share that pin.
- **`docs/MCP_SCHEMA.md` is generated** — `scripts/check-schema-docs.py --check` runs in CI and fails on drift after any `ACTION_SPECS` change.


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
