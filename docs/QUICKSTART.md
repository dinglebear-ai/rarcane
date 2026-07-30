---
title: "rarcane Quickstart"
created: "2026-07-30"
updated: "2026-07-30"
---

# rarcane Quickstart

## 1. Configure Arcane

```bash
export RARCANE_API_URL=https://arcane.example.com
export RARCANE_API_KEY=change-me
export RARCANE_MCP_HOST=127.0.0.1
export RARCANE_MCP_PORT=40110
export RARCANE_MCP_NO_AUTH=true
```

## 2. Try the CLI

```bash
cargo run -- status
cargo run -- help --domain container
cargo run -- call --action container --subaction list --env-id default
```

Destructive operations need confirmation:

```bash
cargo run -- call --action container --subaction stop --env-id default --id nginx --confirm
```

## 3. Start HTTP MCP

```bash
cargo run -- serve
```

```bash
curl -s -X POST http://127.0.0.1:40110/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"arcane","arguments":{"action":"status"}}}'
```

## 4. Verify

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
```
