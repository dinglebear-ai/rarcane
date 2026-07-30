# arcane-rmcp

MCP server and CLI for Arcane: manage Docker containers, images, networks, volumes, and Compose projects over stdio or streamable HTTP, with auth.

It exposes one MCP tool, `arcane`, plus the `rarcane` CLI. Agents can inspect
Arcane environments, manage compose projects, containers, images, networks,
volumes, registries, GitOps syncs, image updates, vulnerability findings, and
system operations through stdio MCP, Streamable HTTP MCP, or direct shell
commands.

**30-second path:** set `RARCANE_API_URL` and `RARCANE_API_KEY`, then run
`npx -y arcane-rmcp status` -> start loopback HTTP with
`RARCANE_MCP_HOST=127.0.0.1 npx -y arcane-rmcp serve` -> call `tools/call` with
`{"action":"status"}`.

**Status:** operational RMCP upstream-client server. Write-capable; destructive
Docker and Arcane operations require explicit confirmation. HTTP MCP supports
loopback dev mode, static bearer tokens, and Google OAuth through `lab-auth`.

**Not for:** replacing Arcane, bypassing Docker or Arcane authorization,
running arbitrary shell commands, storing registry or Git credentials,
multi-tenant isolation, or passing Arcane API keys through MCP tool arguments.

## Contents

- [Naming](#naming)
- [Capabilities And Boundaries](#capabilities-and-boundaries)
- [Install](#install)
- [Quickstart](#quickstart)
- [Client Configuration](#client-configuration)
- [Runtime Surfaces](#runtime-surfaces)
- [MCP Tool Reference](#mcp-tool-reference)
- [CLI Reference](#cli-reference)
- [Configuration](#configuration)
- [Authentication](#authentication)
- [Safety And Trust Model](#safety-and-trust-model)
- [Architecture](#architecture)
- [Distribution Contract](#distribution-contract)
- [Development](#development)
- [Verification](#verification)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Related Servers](#related-servers)
- [Documentation](#documentation)
- [License](#license)

## Naming

| Surface | This repo |
|---|---|
| Repository | `arcane-rmcp` |
| Rust crate | `rarcane` |
| Binary / CLI | `rarcane` |
| npm package | `arcane-rmcp` |
| npm binary alias | `rarcane` |
| MCP server name | `rarcane` in bundled plugin/client config |
| MCP tool | `arcane` |
| Config home | `~/.rarcane` on hosts, `/data` in containers |
| Env prefixes | `RARCANE_*`, `RARCANE_MCP_*`, `RARCANE_RMCP_*` for npm launcher controls |

The repo and npm package use the upstream service name, while the shipped
binary keeps the historical Rust CLI name `rarcane`. The MCP server may be
registered as `rarcane`, but the tool clients call is `arcane`.

## Capabilities And Boundaries

- Read Arcane status plus Docker environment, project, container, image,
  network, volume, registry, GitOps, update, vulnerability, and system state.
- Create, update, start, stop, restart, delete, prune, deploy, sync, scan, and
  back up supported Arcane resources through action/subaction dispatch.
- Enforce action scopes and destructive-operation confirmation before forwarding
  write operations to Arcane.
- Expose the `quick_start` prompt and `rarcane://schema/mcp-tool` resource for
  client-side discovery.
- Provide setup, doctor, and watch commands for local plugin/runtime checks.

| This repo owns | Arcane owns | Explicitly out of scope |
|---|---|---|
| MCP/CLI projection, request validation, auth policy, response shaping, setup checks, schema/resource exposure, and destructive gates. | Docker state, Arcane projects and environments, upstream authorization, registry credentials, GitOps secrets, vulnerability scanner output, and API semantics. | Direct Docker socket access, shell execution, credential storage, generic REST proxy behavior, multi-tenant sandboxing, scheduler behavior, and replacing the Arcane UI/API. |

## Install

| Path | Command | Best for | Notes |
|---|---|---|---|
| npm / npx | `npx -y arcane-rmcp --help` | Local MCP clients and quick trials. | Downloads the matching `rarcane` binary from GitHub Releases. |
| Release installer | `curl -fsSL https://raw.githubusercontent.com/dinglebear-ai/rarcane/main/scripts/install.sh \| bash` | Host installs without Node. | Installs `rarcane` for the current Linux host. |
| Docker / Compose | `docker compose up -d` | Shared HTTP MCP deployments. | Reads `.env` and exposes container port `40110`. |
| Build from source | `cargo build --release` | Development and audits. | Produces `target/release/rarcane`. |
| Plugin | `claude plugin install plugins/rarcane` | Claude Code local plugin setup from this checkout. | Uses the packaged skill and monitor metadata. The plugin ships no lifecycle hooks — run `rarcane setup repair` once after install. |

### npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y arcane-rmcp --help
npx -y arcane-rmcp mcp
npx -y arcane-rmcp status
```

The npm package downloads `rarcane` during `postinstall`. Override download
behavior only when testing packaging:

| Variable | Purpose |
|---|---|
| `RARCANE_RMCP_SKIP_DOWNLOAD=1` | Skip postinstall binary download. |
| `RARCANE_RMCP_VERSION` or `RARCANE_RMCP_BINARY_VERSION` | Select the GitHub Release tag. |
| `RARCANE_RMCP_REPO` | Select the GitHub repo used for release downloads. |
| `RARCANE_RMCP_RELEASE_BASE_URL` | Select a custom release base URL. |

### Build From Source

```bash
git clone https://github.com/dinglebear-ai/rarcane
cd rarcane
cargo build --release
./target/release/rarcane --help
```

Minimum supported Rust version: 1.90.

## Quickstart

### 1. Configure Arcane

Point the bridge at an existing Arcane API server:

```bash
export RARCANE_API_URL=https://arcane.example.com
export RARCANE_API_KEY=...
```

The API key is read from env or config only. Do not pass it in MCP tool
arguments or CLI `--params-json`.

### 2. Run A Safe CLI Call

```bash
npx -y arcane-rmcp status
```

### 3. Start Loopback HTTP MCP

```bash
RARCANE_MCP_HOST=127.0.0.1 npx -y arcane-rmcp serve
```

In another shell:

```bash
curl -sf http://127.0.0.1:40110/health
```

### 4. Make A First MCP Call

```bash
curl -s -X POST http://127.0.0.1:40110/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"arcane","arguments":{"action":"status"}}}'
```

## Client Configuration

### Claude Code Stdio

```json
{
  "mcpServers": {
    "rarcane": {
      "command": "npx",
      "args": ["-y", "arcane-rmcp", "mcp"],
      "env": {
        "RARCANE_API_URL": "https://arcane.example.com",
        "RARCANE_API_KEY": "..."
      }
    }
  }
}
```

### Claude Code HTTP

```json
{
  "mcpServers": {
    "rarcane": {
      "type": "http",
      "url": "http://127.0.0.1:40110/mcp",
      "headers": {
        "Authorization": "Bearer ${RARCANE_MCP_TOKEN}"
      }
    }
  }
}
```

### Codex / Labby Gateway

Register Arcane through Labby as an HTTP upstream when sharing one long-running
server, or run it directly as stdio for local-only use.

```toml
[mcp_servers.rarcane]
command = "npx"
args = ["-y", "arcane-rmcp", "mcp"]
```

### Generic MCP JSON

```json
{
  "command": "rarcane",
  "args": ["mcp"],
  "env": {
    "RARCANE_API_URL": "https://arcane.example.com"
  }
}
```

Do not put `RARCANE_API_KEY`, registry credentials, Git credentials, OAuth
secrets, SSH keys, passwords, or upstream bearer tokens in MCP tool arguments.
Use env, config files, or the MCP client's secret storage.
MCP callers never provide credentials, tokens, keys, or secrets as action
arguments.

## Runtime Surfaces

| Surface | Status | Entry point | Purpose |
|---|---:|---|---|
| MCP stdio | Supported | `rarcane mcp`, `npx -y arcane-rmcp mcp` | Local child-process MCP clients. |
| MCP HTTP | Supported | `rarcane serve`, `POST /mcp` | Streamable HTTP MCP for local or shared server deployments. |
| CLI | Supported | `rarcane <command>` | Scriptable parity and debugging. |
| Prompt | Supported | `quick_start` | Guides a client through `status` and public `help`. |
| Resource | Supported | `rarcane://schema/mcp-tool` | JSON schema for the `arcane` tool. |
| REST API | Not shipped | N/A | Arcane already owns the REST API. |
| Web UI | Not shipped | N/A | Arcane already owns the web UI. |

## MCP Tool Reference

One MCP tool is exposed: `arcane`. Pass the required `action` argument and, for
most domains, a required `subaction`.

| Action | Common subactions | Scope |
|---|---|---|
| `help` | action reference or domain-specific help | public |
| `status` | local bridge and Arcane config status | `rarcane:read` |
| `elicit_name` | MCP elicitation demonstration | `rarcane:read` |
| `scaffold_intent` | side-effect-free scaffold planning handoff | `rarcane:read` |
| `environment` | `list`, `get`, `create`, `update`, `delete`, `test` | read/write |
| `project` | `list`, `get`, `create`, `update`, `up`, `down`, `restart`, `pull`, `destroy`, `redeploy`, `build` | read/write |
| `container` | `list`, `get`, `create`, `start`, `stop`, `restart`, `update`, `delete`, `stats` | read/write |
| `image` | `list`, `get`, `pull`, `delete`, `prune`, `scan` | read/write |
| `network` | `list`, `get`, `create`, `delete`, `prune` | read/write |
| `volume` | `list`, `get`, `create`, `delete`, `prune`, `browse`, `list-backups`, `create-backup`, `delete-backup`, `restore`, `restore-files` | read/write |
| `system` | `docker-info`, `prune`, `start-all`, `stop-all`, `convert` | read/write |
| `image-update` | `check-all`, `check`, `check-batch`, `summary` | read |
| `vulnerability` | `summary`, `list`, `scanner-status`, `ignore`, `unignore`, `list-ignored` | read/write |
| `registry` | `list`, `get`, `create`, `update`, `delete`, `test` | read/write |
| `gitops` | `list`, `get`, `create`, `update`, `delete`, `sync`, `status`, `browse` | read/write |

Action specs in `src/actions.rs` and the generated schema notes in
`docs/MCP_SCHEMA.md` are the source of truth for required parameters.

Example read-only tool arguments:

```json
{
  "action": "container",
  "subaction": "list",
  "envId": "default"
}
```

Example destructive operation:

```json
{
  "action": "container",
  "subaction": "stop",
  "envId": "default",
  "id": "my-container",
  "params": {
    "confirm": true
  }
}
```

## CLI Reference

`rarcane` exposes the same service layer as the MCP tool:

```bash
rarcane status
rarcane help --domain container
rarcane call --action container --subaction list --env-id default
rarcane call --action system --subaction docker-info --env-id default
rarcane call --action container --subaction stop --env-id default --id my-container --confirm
rarcane doctor --json
rarcane watch --url http://127.0.0.1:40110
rarcane setup check
rarcane setup repair
```

`--params-json` accepts action-specific JSON payloads. Do not use it to pass
credentials.

## Configuration

Host installs read `~/.rarcane/.env`, `~/.rarcane/config.toml`, and process env.
Containers read `/data/.env`, `/data/config.toml`, and process env.

| Variable | Default | Purpose |
|---|---|---|
| `RARCANE_API_URL` | unset | Arcane API base URL. |
| `RARCANE_API_KEY` | unset | Arcane API key or bearer token. |
| `RARCANE_MCP_HOST` | `127.0.0.1` | HTTP bind host. |
| `RARCANE_MCP_PORT` | `40110` | HTTP bind port. |
| `RARCANE_MCP_SERVER_NAME` | `arcane-rmcp` | Advertised MCP server name. |
| `RARCANE_MCP_TOKEN` | unset | Static bearer token for HTTP MCP. |
| `RARCANE_MCP_NO_AUTH` | `false` | Disable auth only for loopback development. |
| `RARCANE_NOAUTH` | `false` | Trust an upstream gateway to enforce auth. |
| `RARCANE_MCP_ALLOWED_HOSTS` | unset | Extra accepted Host header values. |
| `RARCANE_MCP_ALLOWED_ORIGINS` | unset | Extra accepted CORS origins. |
| `RARCANE_MCP_PUBLIC_URL` | unset | Public base URL for OAuth issuer metadata. |
| `RARCANE_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth`. |
| `RARCANE_MCP_ALLOW_DESTRUCTIVE` | `false` | Bypass the per-call destructive confirmation gate. Do not set in shared deployments. |
| `RARCANE_HOME` | `~/.rarcane` | Config/appdata root override. |
| `RUST_LOG` | `info` | Log filter. |

## Authentication

Stdio MCP runs as a local trusted child process and does not use HTTP auth.

HTTP MCP auth policy:

| State | Condition | Behavior |
|---|---|---|
| Loopback dev | Bound to `127.0.0.1`, `localhost`, or `[::1]` | Local unauthenticated development is allowed. |
| Mounted bearer | Non-loopback with `RARCANE_MCP_TOKEN` | Requires `Authorization: Bearer <token>` and action scopes. |
| Mounted OAuth | `RARCANE_MCP_AUTH_MODE=oauth` | Uses Google OAuth/JWT through `lab-auth`; static bearer remains supported. |
| Trusted gateway | `RARCANE_NOAUTH=true` | Assumes a reverse proxy or gateway already enforced auth. |

OAuth mode uses the `RARCANE_MCP_*` Google OAuth variables documented in
`docs/CONFIG.md`.

OAuth mode is single-replica: client/session state and signing keys are local to
one host. Do not run multiple OAuth replicas until those stores are externalized
or safely shared.

## Safety And Trust Model

- Arcane API credentials are loaded from config/env only.
- MCP callers select actions and payloads, not upstream credentials.
- Destructive subactions reject unless `params.confirm=true` or CLI
  `--confirm` is present.
- Unknown actions, unknown subactions, missing environment IDs, missing IDs, and
  unsafe volume paths are rejected before upstream calls.
- Non-loopback HTTP deployments must use bearer auth, OAuth, or a trusted
  authenticated gateway.
- This bridge does not sandbox Arcane itself. Arcane remains responsible for
  Docker authorization and the effects of Docker operations.

## Architecture

```text
ArcaneClient  (src/arcane.rs)      HTTP transport and redacted errors
      |
ArcaneService (src/app.rs)         action validation, confirmation, response normalization
      |
MCP shim      (src/mcp/tools.rs)   JSON args -> service -> Value
CLI shim      (src/cli.rs)         argv -> service -> stdout
```

## Distribution Contract

- `Cargo.toml`, `Cargo.lock`, `packages/arcane-rmcp/package.json`,
  `.release-please-manifest.json`, and `server.json` must agree on the released
  version.
- GitHub Releases publish the `rarcane` binary consumed by the npm launcher.
- The npm package name is `arcane-rmcp`; the installed binary alias is
  `rarcane`.
- Docker/OCI metadata uses `ghcr.io/dinglebear-ai/rarcane:<version>`.
- `plugins/rarcane/.mcp.json` must launch `npx -y arcane-rmcp mcp` so stdio
  clients start the MCP transport rather than the HTTP server.
- The root README is curated. Generated or source-of-truth details live in
  `src/actions.rs`, `docs/MCP_SCHEMA.md`, and the package/registry manifests.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
npm --prefix packages/arcane-rmcp run check
```

## Verification

```bash
just verify                 # fmt-check → lint → check → test
just template-check         # patterns, plugin layout, schema docs, scaffold contract
npm --prefix packages/arcane-rmcp run check
git diff --check
```

Runtime smoke:

```bash
RARCANE_API_URL=https://arcane.example.com \
RARCANE_API_KEY=... \
rarcane status
```

HTTP smoke:

```bash
RARCANE_MCP_HOST=127.0.0.1 rarcane serve
curl -sf http://127.0.0.1:40110/health
```

## Deployment

Use loopback for local development:

```bash
RARCANE_MCP_HOST=127.0.0.1 rarcane serve
```

Use Docker Compose for shared HTTP deployment:

```bash
cp .env.example .env
docker compose up -d
```

When binding to a non-loopback address, configure `RARCANE_MCP_TOKEN`,
`RARCANE_MCP_AUTH_MODE=oauth`, or `RARCANE_NOAUTH=true` behind an authenticated
gateway.

## Troubleshooting

| Symptom | Check |
|---|---|
| `RARCANE_API_URL is required` | Set `RARCANE_API_URL` in env or `~/.rarcane/.env`. |
| Arcane calls return unauthorized | Refresh `RARCANE_API_KEY` in Arcane and restart the bridge. |
| HTTP `/mcp` returns unauthorized | Set `RARCANE_MCP_TOKEN` and send `Authorization: Bearer <token>`. |
| Stdio client hangs or logs JSON errors | Ensure client config runs `arcane-rmcp mcp`, not the default HTTP server mode. |
| Destructive action is rejected | Add CLI `--confirm` or MCP `params.confirm=true` after verifying the target. |
| Port conflict | Set `RARCANE_MCP_PORT` or stop the process already using `40110`. |

## Related Servers

- [soma](https://github.com/dinglebear-ai/soma) - RMCP runtime for provider-backed MCP servers.
- [unifi-rmcp](https://github.com/dinglebear-ai/runifi) - UniFi controller REST API bridge.
- [tailscale-rmcp](https://github.com/dinglebear-ai/rtailscale) - Tailscale API bridge for devices, users, and tailnet operations.
- [unraid](https://github.com/dinglebear-ai/unraid) - Unraid GraphQL bridge for NAS and server management.
- [apprise-rmcp](https://github.com/dinglebear-ai/rapprise) - Apprise notification fan-out bridge for many delivery backends.
- [gotify-rmcp](https://github.com/dinglebear-ai/rgotify) - Gotify push notification bridge for sends, messages, apps, and clients.
- [yarr](https://github.com/dinglebear-ai/yarr) - Media-stack bridge for Sonarr, Radarr, Prowlarr, Plex, and related services.
- [ytdl-rmcp](https://github.com/dinglebear-ai/rytdl) - Media download and metadata workflow server.
- [synapse-rmcp](https://github.com/dinglebear-ai/synapse) - Local Synapse workflow server for scout and flux actions.
- [cortex](https://github.com/dinglebear-ai/cortex) - Syslog and homelab log aggregation MCP server.
- [axon](https://github.com/dinglebear-ai/axon) - RAG, crawl, scrape, extract, and semantic search project.
- [labby](https://github.com/dinglebear-ai/labby) - Homelab control plane and MCP gateway project.
- [lumen](https://github.com/dinglebear-ai/lumen) - Local semantic code search MCP server.

## Documentation

- `docs/API.md` is the curated action-contract overview.
- `docs/CONFIG.md` is the curated configuration and auth reference.
- `docs/QUICKSTART.md` is the curated smoke-test guide.
- `docs/MCP_SCHEMA.md` is the generated/schema-drift contract for actions,
  resources, prompts, and validation rules.
- `plugins/rarcane/skills/rarcane/SKILL.md` is the agent usage guide.

## License

MIT. See [LICENSE](LICENSE).
