//! Configuration structs for the Arcane MCP server.
//!
//! Values are loaded in priority order:
//!   1. `config.toml` (checked in, defaults only — no secrets)
//!   2. Environment variables (`RARCANE_*`, `RARCANE_MCP_*`)
//!
//! **Template**: rename `ArcaneConfig` to match your service. Adjust env prefixes
//! throughout. Add any domain-specific config fields you need.

use serde::{Deserialize, Serialize};

/// TEMPLATE: Replace with your service name (e.g. ".unraid", ".gotify").
const SERVICE_HOME_DIRNAME: &str = ".rarcane";

/// Top-level config (maps to `config.toml` sections).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub mcp: McpConfig,
    pub rarcane: ArcaneConfig,
}

/// Config for the rarcane remote service (the thing this MCP server wraps).
///
/// **Template**: replace this with config for your actual upstream service.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ArcaneConfig {
    /// Full endpoint URL of the remote service (RARCANE_API_URL).
    /// Arcane: `https://api.rarcane.com/v1`
    pub api_url: String,
    /// API key or bearer token (RARCANE_API_KEY).
    pub api_key: String,
}

/// MCP HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct McpConfig {
    /// Bind host (RARCANE_MCP_HOST). Default: `127.0.0.1` (loopback).
    /// Set to `0.0.0.0` to listen on all interfaces — requires auth configured.
    #[serde(default = "default_mcp_host")]
    pub host: String,
    /// Bind port (RARCANE_MCP_PORT). Default: `40110`.
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    /// MCP server name advertised to clients (RARCANE_MCP_SERVER_NAME).
    #[serde(default = "default_server_name")]
    pub server_name: String,
    /// Disable auth entirely — only safe when bound to loopback (RARCANE_MCP_NO_AUTH).
    pub no_auth: bool,
    /// Allow unauthenticated access on non-loopback when behind a trusted reverse proxy
    /// that enforces its own auth (RARCANE_NOAUTH). Loaded here so it participates in
    /// typed config rather than being a raw env read at call sites.
    pub trusted_gateway: bool,
    /// Static bearer token for simple auth (RARCANE_MCP_TOKEN).
    pub api_token: Option<String>,
    /// Additional allowed Host header values (comma-separated in env).
    pub allowed_hosts: Vec<String>,
    /// Additional allowed CORS origins (comma-separated in env).
    pub allowed_origins: Vec<String>,
    /// OAuth sub-config (nested under `[mcp.auth]` in config.toml).
    pub auth: AuthConfig,
}

impl McpConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Return true if the configured bind host resolves to a loopback address.
    ///
    /// Uses `IpAddr::is_loopback()` for numeric addresses. Accepts "localhost"
    /// as a canonical loopback hostname. Any other hostname or parse failure is
    /// treated as non-loopback — callers must not assume safety in that case.
    pub fn is_loopback(&self) -> bool {
        let host = &self.host;
        // Match "localhost" literal and numeric loopback addresses.
        // Strip bracket notation ([::1]) before parsing so IPv6 loopback works.
        host == "localhost"
            || host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<std::net::IpAddr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false)
    }
}

/// OAuth / JWT auth sub-config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    pub mode: AuthMode,
    /// Disable the static bearer token when OAuth is active. This supports
    /// migration periods where both mechanisms coexist without forcing them.
    pub disable_static_token_with_oauth: bool,
    pub public_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub admin_email: String,
    pub allowed_emails: Vec<String>,
    pub sqlite_path: String,
    pub key_path: String,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub auth_code_ttl_secs: u64,
    pub register_rpm: u32,
    pub authorize_rpm: u32,
    pub allowed_client_redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    #[default]
    Bearer,
    OAuth,
}

// ── defaults ──────────────────────────────────────────────────────────────────

fn default_mcp_host() -> String {
    // Default to loopback for safety. Operators who need external access must
    // explicitly set RARCANE_MCP_HOST=0.0.0.0 (and configure auth).
    "127.0.0.1".into()
}
fn default_mcp_port() -> u16 {
    40110
}
fn default_server_name() -> String {
    "arcane-rmcp".into()
}
fn default_auth_sqlite_path() -> String {
    "/data/auth.db".into()
}
fn default_auth_key_path() -> String {
    "/data/auth-jwt.pem".into()
}
fn default_access_token_ttl_secs() -> u64 {
    3600
}
fn default_refresh_token_ttl_secs() -> u64 {
    86400 * 30
}
fn default_auth_code_ttl_secs() -> u64 {
    300
}
fn default_register_rpm() -> u32 {
    10
}
fn default_authorize_rpm() -> u32 {
    60
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            host: default_mcp_host(),
            port: default_mcp_port(),
            server_name: default_server_name(),
            no_auth: false,
            trusted_gateway: false,
            api_token: None,
            allowed_hosts: Vec::new(),
            allowed_origins: Vec::new(),
            auth: AuthConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::default(),
            disable_static_token_with_oauth: false,
            public_url: None,
            google_client_id: None,
            google_client_secret: None,
            admin_email: String::new(),
            allowed_emails: Vec::new(),
            sqlite_path: default_auth_sqlite_path(),
            key_path: default_auth_key_path(),
            access_token_ttl_secs: default_access_token_ttl_secs(),
            refresh_token_ttl_secs: default_refresh_token_ttl_secs(),
            auth_code_ttl_secs: default_auth_code_ttl_secs(),
            register_rpm: default_register_rpm(),
            authorize_rpm: default_authorize_rpm(),
            allowed_client_redirect_uris: Vec::new(),
        }
    }
}

// ── Appdata directory ─────────────────────────────────────────────────────────

/// Return the default local data directory for this service.
///
/// Pattern §25 + §28: The same `.env` and `config.toml` in `~/.<service>/`
/// work for both Docker and bare-metal deployment without modification.
///
/// | Environment   | Path                                |
/// |---------------|-------------------------------------|
/// | Container     | `/data` (bind-mounted from host)     |
/// | Bare-metal    | `~/.rarcane` (user home dir)        |
///
/// TEMPLATE: Replace `.rarcane` with your service name (e.g. `.unraid`, `.gotify`).
///           The name should match the docker-compose.yml volume mount source.
pub fn default_data_dir() -> anyhow::Result<std::path::PathBuf> {
    // Running inside a Docker container — /data is always the mount point.
    // Detection uses /.dockerenv (created by the Docker runtime) or an explicit
    // RUNNING_IN_CONTAINER env var (useful for testing or systemd-nspawn).
    if std::path::Path::new("/.dockerenv").exists()
        || std::env::var("RUNNING_IN_CONTAINER").is_ok()
        || std::env::var("container").is_ok()
    {
        return Ok(std::path::PathBuf::from("/data"));
    }

    // Bare-metal or local dev — use ~/.<service>/
    let home = dirs::home_dir().ok_or_else(|| {
        anyhow::anyhow!("cannot determine home directory — set HOME or RUNNING_IN_CONTAINER=1")
    })?;
    Ok(home.join(SERVICE_HOME_DIRNAME))
}

/// Load `~/.rarcane/.env` (or `/data/.env` in a container) into the process
/// environment if present.
///
/// Best-effort: a missing file is ignored, and existing env vars are NOT
/// overridden — values injected by docker-compose/systemd or the plugin hook's
/// `CLAUDE_PLUGIN_OPTION_*` mapping still take precedence. Lets the binary find
/// its credentials directly from `~/.rarcane/.env` without a process manager.
/// Call once at startup before `Config::load`. A symlinked `.env` is refused
/// (the dir holds secrets; mirrors axon).
pub fn load_dotenv() -> anyhow::Result<()> {
    let dir = default_data_dir()?;
    let env_path = dir.join(".env");
    match std::fs::symlink_metadata(&env_path) {
        Ok(md) if md.file_type().is_symlink() => {
            anyhow::bail!(
                "refusing to load symlinked .env at {} (potential symlink attack)",
                env_path.display()
            );
        }
        Ok(_) => {
            dotenvy::from_path(&env_path).map_err(|error| {
                anyhow::anyhow!("failed to load {}: {error}", env_path.display())
            })?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(anyhow::anyhow!(
                "failed to inspect {}: {error}",
                env_path.display()
            ));
        }
    }
    Ok(())
}

// ── Config loading ────────────────────────────────────────────────────────────

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        // Search for config.toml in priority order (§25: appdata convention):
        //   1. ~/<SERVICE_HOME_DIRNAME>/config.toml  — user's persistent config (primary)
        //   2. ./config.toml                         — local dev / Docker mount fallback
        let candidate_paths = {
            let mut paths = vec![];
            if let Some(home) = std::env::var_os("HOME") {
                paths.push(
                    std::path::PathBuf::from(home)
                        .join(SERVICE_HOME_DIRNAME)
                        .join("config.toml"),
                );
            }
            paths.push(std::path::PathBuf::from("config.toml"));
            paths
        };

        for path in &candidate_paths {
            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    config = toml::from_str(&contents)
                        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {e}", path.display()))?;
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(anyhow::anyhow!("Failed to read {}: {e}", path.display())),
            }
        }

        // Env overrides — RARCANE_MCP_* for server config, RARCANE_API_* for upstream
        env_str("RARCANE_MCP_HOST", &mut config.mcp.host);
        env_parse("RARCANE_MCP_PORT", &mut config.mcp.port)?;
        env_str("RARCANE_MCP_SERVER_NAME", &mut config.mcp.server_name);
        env_bool("RARCANE_MCP_NO_AUTH", &mut config.mcp.no_auth)?;
        env_bool("RARCANE_NOAUTH", &mut config.mcp.trusted_gateway)?;
        env_opt_str("RARCANE_MCP_TOKEN", &mut config.mcp.api_token);
        env_list("RARCANE_MCP_ALLOWED_HOSTS", &mut config.mcp.allowed_hosts);
        env_list(
            "RARCANE_MCP_ALLOWED_ORIGINS",
            &mut config.mcp.allowed_origins,
        );
        env_opt_str("RARCANE_MCP_PUBLIC_URL", &mut config.mcp.auth.public_url);
        env_str(
            "RARCANE_MCP_AUTH_ADMIN_EMAIL",
            &mut config.mcp.auth.admin_email,
        );
        env_opt_str(
            "RARCANE_MCP_GOOGLE_CLIENT_ID",
            &mut config.mcp.auth.google_client_id,
        );
        env_opt_str(
            "RARCANE_MCP_GOOGLE_CLIENT_SECRET",
            &mut config.mcp.auth.google_client_secret,
        );
        if let Ok(v) = std::env::var("RARCANE_MCP_AUTH_MODE")
            && !v.is_empty()
        {
            config.mcp.auth.mode = match v.to_lowercase().as_str() {
                "oauth" => AuthMode::OAuth,
                "bearer" => AuthMode::Bearer,
                other => {
                    return Err(anyhow::anyhow!(
                        "invalid RARCANE_MCP_AUTH_MODE {:?}: must be \"bearer\" or \"oauth\"",
                        other
                    ));
                }
            };
        }

        // Upstream service config
        env_str("RARCANE_API_URL", &mut config.rarcane.api_url);
        env_str("RARCANE_API_KEY", &mut config.rarcane.api_key);

        Ok(config)
    }
}

// ── env helpers ───────────────────────────────────────────────────────────────

fn env_str(key: &str, target: &mut String) {
    if let Ok(v) = std::env::var(key)
        && !v.is_empty()
    {
        *target = v;
    }
}

fn env_opt_str(key: &str, target: &mut Option<String>) {
    if let Ok(v) = std::env::var(key)
        && !v.is_empty()
    {
        *target = Some(v);
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, target: &mut T) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key)
        && !v.is_empty()
    {
        *target = v
            .parse()
            .map_err(|_| anyhow::anyhow!("{key}: invalid value {v:?}"))?;
    }
    Ok(())
}

fn env_bool(key: &str, target: &mut bool) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key) {
        match v.to_lowercase().as_str() {
            "1" | "true" | "yes" => *target = true,
            "0" | "false" | "no" => *target = false,
            other => anyhow::bail!("{key}: expected bool, got {other:?}"),
        }
    }
    Ok(())
}

fn env_list(key: &str, target: &mut Vec<String>) {
    if let Ok(v) = std::env::var(key) {
        let items: Vec<String> = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !items.is_empty() {
            *target = items;
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
