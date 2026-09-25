# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- TEMPLATE: When releasing, move items from [Unreleased] to a new version section.
               Format: ## [X.Y.Z] — YYYY-MM-DD
               Use Added / Changed / Deprecated / Removed / Fixed / Security headers. -->

## [0.4.4](https://github.com/jmagar/rarcane/compare/v0.4.3...v0.4.4) (2026-07-18)


### Fixed

* **release:** install LLVM archive tooling and NASM for Windows artifacts ([16d5873](https://github.com/jmagar/rarcane/commit/16d58738cc2db3137de2303bfbdebe6f85fc6e6d))
* **release:** use the draft-owner token for asset staging and finalization ([e07a3a2](https://github.com/jmagar/rarcane/commit/e07a3a2d54e001ee7470cdef5ffb3e399d63ba64))

## [0.4.3](https://github.com/jmagar/rarcane/compare/v0.4.2...v0.4.3) (2026-07-18)

### Fixed

* Build draft releases from an explicit source commit instead of a tag that does not exist yet.
* Keep npm, Cargo, and MCP registry versions synchronized and fail closed on drift.
* **ci:** allow cold multi-arch image builds ([061b30b](https://github.com/jmagar/rarcane/commit/061b30bc10f6d1176672333830f450f2e721775e))
* **ci:** repair invalid action pins ([5115543](https://github.com/jmagar/rarcane/commit/511554333f5b40f81e5937e192c2eb60f689d993))
* **ci:** upgrade Trivy scanner action ([eddf4ca](https://github.com/jmagar/rarcane/commit/eddf4ca44fc72e73e57f6bfa43a5a092c1908294))
* **docker:** isolate multi-arch cargo caches ([38e281a](https://github.com/jmagar/rarcane/commit/38e281a26476bf55a6f4b7b629a57a3cd2630784))

## [0.4.2](https://github.com/jmagar/rarcane/compare/v0.4.1...v0.4.2) (2026-07-18)


### Fixed

* address lavra review findings ([56d53d8](https://github.com/jmagar/rarcane/commit/56d53d84156839c15659d985209785c4c54dbac7))
* migrate MCP models to rmcp 2.2 ([a65ed85](https://github.com/jmagar/rarcane/commit/a65ed8532baf070f77180c6b9f4416fd2bf72a2d))
* remediate comprehensive repository review ([de496d1](https://github.com/jmagar/rarcane/commit/de496d104a632feedcec0818256ac9fa698f94a3))
* remediate comprehensive review findings ([5e6398d](https://github.com/jmagar/rarcane/commit/5e6398d96560c8f498e42059cc937447d32aef92))
* route rust builds through sccache wrapper ([e9ceeb1](https://github.com/jmagar/rarcane/commit/e9ceeb17e59d45a512ae66c7d930a63d86cacc14))
* update vulnerable cmov dependency ([b6bda37](https://github.com/jmagar/rarcane/commit/b6bda37a9457ee34c6e2b579902189214fa4a7a0))
* validate staged npm release assets ([1c5a787](https://github.com/jmagar/rarcane/commit/1c5a7877f26278b404168520ae00d662de698e2b))

### Changed

* Published the npm launcher at `0.4.2`.

## [Unreleased]

### Changed

- Reconciled the active documentation with the MCP + CLI upstream-client
  architecture; removed stale REST, Web, and OpenAPI claims.
- Pattern checks now fail closed on file-read and directory-traversal errors.
- Corrected setup, deployment, observability, OAuth topology, and prompt guidance.

## [0.4.1] — 2026-07-18

### Changed

- Published the Rust crate and binary metadata at `0.4.1`.
- Replaced the original greeting/echo scaffold with the Arcane domain action
  registry and MCP/CLI dispatch surface.

## [0.4.0] — 2026-05-14

### Added

- `.github/workflows/codeql.yml` — CodeQL SAST analysis on push to main and weekly scheduled scan; results surface in the GitHub Security tab.
- `.github/workflows/cargo-deny.yml` — license compliance, duplicate dependency, advisory, and source checks via `cargo-deny`.
- `.github/workflows/msrv.yml` — compiles against the declared `rust-version` to catch MSRV regressions early.

## [0.3.0] — 2026-05-14

### Added

- `src/cli/watch.rs` — `rarcane watch` subcommand for live file-system monitoring.
- `plugins/rarcane/monitors/` — plugin monitor definitions for event-driven automation.
- `plugins/rarcane/gemini-extension.json` — Gemini extension manifest for multi-platform plugin distribution.
- `.github/dependabot.yml` + `.github/workflows/dependabot-auto-merge.yml` — automated dependency updates with auto-merge for minor/patch bumps.
- `scripts/asciicheck.py`, `scripts/check-blob-size.py`, `scripts/check-dependency-updates.sh`, `scripts/check-file-size.sh`, `scripts/check-runtime-current.sh`, `scripts/validate-plugin-layout.sh`, `scripts/blob-size-allowlist.txt` — repository validation and quality scripts.
- `tests/plugin_contract.rs` — plugin contract integration tests.
- `docs/PLUGINS.md` — documentation for the plugin system and distribution model.
- `plugins/README.md`, `plugins/rarcane/README.md`, `plugins/rarcane/CLAUDE.md` — plugin-level documentation and agent guidance.
- `xtask/README.md`, `tests/README.md`, and `scripts/README.md` — focused automation and test documentation.
- `.claude/` — Claude Code project settings for agent-assisted development.

### Changed

- `plugins/rarcane/hooks/plugin-setup.sh` — significant simplification; reduced from ~500 to ~50 lines by extracting reusable logic and removing duplication.
- `Justfile` — expanded with additional recipes covering plugin validation, script checks, and workflow shortcuts.
- `lefthook.yml` — pre-commit hook additions aligned with new script suite.
- `AGENTS.md`, `CLAUDE.md` — updated agent and AI tooling guidance to reflect current project structure.
- `README.md`, `docs/PATTERNS.md` — documentation refreshed for new scripts and plugin layout.

## [0.2.0] — 2026-05-14

### Changed

- Split HTTP server/auth wiring into `src/server.rs` and `src/server/routes.rs`; `src/mcp/` contains MCP protocol concerns.
- `mcp/rmcp_server.rs` and `mcp/tools.rs` now import `AppState`/`AuthPolicy` from `crate::server` instead of `super`.
- `allowed_origins` visibility widened from `pub(super)` to `pub` to support cross-module access from `server/routes.rs`.
- Updated `src/lib.rs` and `src/main.rs` to reflect the server module layout.

### Added

- `deny.toml` — `cargo-deny` configuration enforcing license allowlist, banning `openssl`/`openssl-sys`, denying yanked crates, and restricting dependency sources to crates.io and `github.com/jmagar/lab.git`. RUSTSEC-2023-0071 acknowledged with rationale.
- `.git/hooks/pre-commit` — enforces the no-`mod.rs` rule at commit time; blocks any staged `mod.rs` file with a clear error message.
- `docs/PATTERNS.md` updated: §1/§1a module layouts reflect new `server`/`api` structure with all `mod.rs` references removed; §5 auth section headers updated; §45 No mod.rs section now includes the git hook script; §A1/§A2 advanced patterns updated to match actual file locations.

### Removed

- `src/mcp/routes.rs` — moved to `src/server/routes.rs`.
- Several obsolete scripts: `backup.sh`, `check-runtime-current.sh`, `plugin-setup.sh`, `reset-db.sh`, `smoke-test.sh`, `test-check-runtime-current.sh`, `validate-marketplace.sh`.
- `docs/server-json-guide.md` — content superseded by `docs/MCP-REGISTRY-PUBLISH-GUIDE.md`.

## [0.1.0] — 2026-05-13

### Added

- Layered architecture: `ArcaneClient` (transport) → `ArcaneService` (business logic) → MCP/CLI shims
- Action-based dispatch: single `rarcane` MCP tool with `action` parameter routing
- Both transports: Streamable HTTP (`rarcane serve`) and stdio (`rarcane mcp`)
- Bearer token authentication via `RARCANE_MCP_TOKEN`
- Google OAuth authentication via `RARCANE_MCP_AUTH_MODE=oauth` (issues RS256 JWTs)
- Loopback/no-auth mode for local development
- MCP resources: exposes tool schema at `rarcane://schema/mcp-tool`
- MCP prompts: `quick_start` prompt
- CLI with `status`, `help`, and generic Arcane `call` commands
- Test helpers: `loopback_state()` and `bearer_state()` for credential-free integration tests
- `AuthPolicy` enum making auth choice explicit at construction time
- CORS, Host header validation, request body size limiting built-in
- `resolve_auth_policy_kind()` — refuses to bind `0.0.0.0` without auth (Pattern §27)
- `default_data_dir()` — detects container vs bare-metal, returns `/data` or `~/.rarcane`
- `entrypoint.sh` — Docker entrypoint with permission setup and privilege drop to UID 1000
- `xtask` crate with `dist`, `ci`, `symlink-docs`, `check-env` commands
- `.config/nextest.toml` — nextest configuration with `default` and `ci` profiles
- `taplo.toml` — TOML formatter configuration
- `lefthook.yml` — minimal pre-commit hooks (diff_check, toml_fmt, env_guard)
- `.github/workflows/ci.yml` — CI: fmt, clippy, nextest, taplo, audit, gitleaks
- `.github/workflows/docker-publish.yml` — multi-platform Docker build + Trivy scan
- `.github/workflows/release.yml` — release binaries for linux/amd64 and linux/arm64
- `config.rarcane.toml` — fully annotated config template
- `.env.rarcane` — documented secrets template
- `CHANGELOG.md` following Keep a Changelog format
- Workspace structure: root crate + `xtask/` member
- `symlink-docs` and `symlink-docs-inline` Justfile recipes
