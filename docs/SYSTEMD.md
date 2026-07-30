---
title: "systemd Deployment"
created: "2026-07-30"
updated: "2026-07-30"
doc_type: "guide"
status: "active"
owner: "rarcane"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
last_reviewed: "2026-05-15"
---

# systemd

The template supports user-level systemd deployments when a unit named `rarcane-mcp.service` is installed by the derived service.

## Install the binary

```bash
cargo build --release
install -m 755 target/release/rarcane ~/.local/bin/rarcane
```

Or use the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/dinglebear-ai/rarcane/main/scripts/install.sh | bash
```

The binary installs to `~/.local/bin/`. Verify it's in `$PATH`:

```bash
rarcane --version
rarcane doctor
```

## Unit file pattern

```ini
[Unit]
Description=rarcane MCP server
After=network.target

[Service]
Type=simple
ExecStart=%h/.local/bin/rarcane serve mcp
Restart=on-failure
RestartSec=5
EnvironmentFile=%h/.rarcane/.env

[Install]
WantedBy=default.target
```

Key points:
- Use `EnvironmentFile` pointing at `~/.rarcane/.env` — never hardcode tokens in unit files.
- `%h` expands to the user home directory.
- `serve mcp` is the canonical Streamable HTTP mode (see `docs/DEPLOYMENT.md`).

## Restart flow

```bash
systemctl --user daemon-reload
systemctl --user restart rarcane-mcp.service
systemctl --user status rarcane-mcp.service
```

## Runtime verification

`just runtime-current` detects stale running processes. The checker compares:

- `/proc/<pid>/exe` for the running service process
- the unit `ExecStart` binary
- optional `--expected-binary`

```bash
scripts/check-runtime-current.sh --mode systemd --expected-binary target/release/rarcane
just runtime-current
```

If hashes differ, install the new binary and restart the unit.

## Logging

With systemd, logs go to the journal:

```bash
journalctl --user -u rarcane-mcp.service -f
journalctl --user -u rarcane-mcp.service --since "1h ago"
```

The current binary logs to stderr, which systemd captures in the journal. The
file-logging helpers are not wired into startup.

## Doctor pre-flight

Run `rarcane doctor` before starting the unit to validate the environment:

```bash
rarcane doctor
```

Exit code 0 = ready to start. Exit code 1 = one or more issues found.

## Environment

Prefer an `EnvironmentFile` that points at the repo or appdata `.env`. See `docs/ENV.md` for variable meanings.

See `docs/PATTERNS.md` §46, §47, §48 for binary commands, installation, and the doctor command patterns.
