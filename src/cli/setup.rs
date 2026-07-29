//! Plugin setup and repair commands.
//!
//! These are operational commands that check and mutate appdata, write .env
//! files, and validate auth/port configuration before the server starts.
//! Business logic stays in `app.rs`; this module is allowed to touch the
//! filesystem and network only for diagnostics and setup purposes.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    config::{default_data_dir, AuthMode, Config},
    server::resolve_auth_policy_kind,
};
use anyhow::{bail, Result};

// ── public surface ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCommand {
    Check,
    Repair,
    /// Copy this binary into ~/.local/bin so it is callable as a bare command
    /// in the user's own terminal, independent of Claude Code.
    Install,
    PluginHook {
        no_repair: bool,
    },
}

/// Translate Claude Code plugin options (`CLAUDE_PLUGIN_OPTION_*`) into the
/// `RARCANE_*` process env vars the binary reads, before `Config::load()` runs.
///
/// This replaces the former `plugin-setup.sh` wrapper: the binary now owns the
/// env-var mapping itself, and the plugin ships no lifecycle hooks. rarcane
/// is template-style — `Config::load()` runs before the setup command dispatches
/// and `setup_check()` validates the pre-loaded `&Config` — so this MUST be
/// called before `Config::load()` (hoisted in `run_cli`, gated to the plugin-hook
/// path). Values containing newlines/CR are skipped, mirroring the script's
/// `reject_unsafe_value` guard.
///
/// No `CLAUDE_PLUGIN_DATA` → `RARCANE_HOME` mapping is needed: `setup_data_dir()`
/// already reads `CLAUDE_PLUGIN_DATA` natively (the script's `RARCANE_HOME`
/// re-export was redundant).
pub fn apply_plugin_options() {
    // CLAUDE_PLUGIN_OPTION_<OPT> -> <RARCANE_ENVVAR>
    let map = [
        ("CLAUDE_PLUGIN_OPTION_API_TOKEN", "RARCANE_MCP_TOKEN"),
        ("CLAUDE_PLUGIN_OPTION_RARCANE_API_URL", "RARCANE_API_URL"),
        ("CLAUDE_PLUGIN_OPTION_RARCANE_API_KEY", "RARCANE_API_KEY"),
        ("CLAUDE_PLUGIN_OPTION_AUTH_MODE", "RARCANE_MCP_AUTH_MODE"),
        ("CLAUDE_PLUGIN_OPTION_NO_AUTH", "RARCANE_MCP_NO_AUTH"),
        ("CLAUDE_PLUGIN_OPTION_PUBLIC_URL", "RARCANE_MCP_PUBLIC_URL"),
        (
            "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID",
            "RARCANE_MCP_GOOGLE_CLIENT_ID",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET",
            "RARCANE_MCP_GOOGLE_CLIENT_SECRET",
        ),
        (
            "CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL",
            "RARCANE_MCP_AUTH_ADMIN_EMAIL",
        ),
    ];
    for (opt, dest) in map {
        if let Some(v) = std::env::var_os(opt) {
            let s = v.to_string_lossy();
            if s.is_empty() || s.contains('\n') || s.contains('\r') {
                continue;
            }
            // edition 2021: set_var is safe (no unsafe block required).
            std::env::set_var(dest, v);
        }
    }
}

pub async fn run_setup(config: &Config, command: SetupCommand) -> Result<()> {
    let report = match command {
        SetupCommand::Check => setup_check(config, true),
        SetupCommand::Repair => setup_repair(config)?,
        SetupCommand::Install => {
            let dest = install_self()?;
            println!("installed -> {}", dest.display());
            return Ok(());
        }
        SetupCommand::PluginHook { no_repair } => setup_plugin_hook(config, no_repair)?,
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.blocking_failures.is_empty() {
        bail!(
            "setup found {} blocking failure(s)",
            report.blocking_failures.len()
        );
    }
    Ok(())
}

// ── internal types ────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct SetupFailure {
    code: &'static str,
    message: String,
}

#[derive(Debug, serde::Serialize)]
struct SetupReport {
    exit_policy: &'static str,
    ran_repair: bool,
    no_repair: bool,
    blocking_failures: Vec<SetupFailure>,
    advisory_failures: Vec<SetupFailure>,
}

impl SetupReport {
    fn new(no_repair: bool) -> Self {
        Self {
            exit_policy: "success",
            ran_repair: false,
            no_repair,
            blocking_failures: Vec::new(),
            advisory_failures: Vec::new(),
        }
    }

    fn finish(mut self) -> Self {
        self.exit_policy = if !self.blocking_failures.is_empty() {
            "blocking_failure"
        } else if !self.advisory_failures.is_empty() {
            "advisory_failure"
        } else {
            "success"
        };
        self
    }
}

// ── setup logic ───────────────────────────────────────────────────────────────

/// Copy the running binary into `~/.local/bin/<name>` so it is callable as a
/// bare command in the user's own terminal, independent of Claude Code.
///
/// Uses the running executable's own file name as the destination, so this is
/// identical across every server repo. Copy (not symlink) so it survives
/// `/plugin update`, which changes the plugin cache path a symlink would dangle
/// to. Atomic via temp + rename; idempotent; depends only on std + anyhow.
fn install_self() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let name = exe
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine binary name from {}", exe.display()))?;
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    let bin_dir = PathBuf::from(home).join(".local").join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    let dest = bin_dir.join(name);

    // Running the already-installed copy: nothing to do.
    if dest == exe {
        return Ok(dest);
    }

    let tmp = bin_dir.join(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::copy(&exe, &tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, &dest).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;

    let on_path = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d == bin_dir))
        .unwrap_or(false);
    if !on_path {
        eprintln!(
            "note: {} is not on your PATH; add:  export PATH=\"$HOME/.local/bin:$PATH\"",
            bin_dir.display()
        );
    }
    Ok(dest)
}

fn setup_plugin_hook(config: &Config, no_repair: bool) -> Result<SetupReport> {
    // Keep the user's terminal copy in ~/.local/bin fresh each session so it
    // survives `/plugin update`. Best-effort: never fail the hook over it.
    if let Err(e) = install_self() {
        eprintln!("setup plugin-hook: self-install skipped: {e}");
    }
    let initial = setup_check(config, no_repair);
    if initial.blocking_failures.is_empty() || no_repair {
        return Ok(initial);
    }
    setup_repair(config)
}

fn setup_check(config: &Config, no_repair: bool) -> SetupReport {
    let mut report = SetupReport::new(no_repair);
    let data_dir = match setup_data_dir() {
        Ok(d) => d,
        Err(e) => {
            report.blocking_failures.push(SetupFailure {
                code: "appdata_dir_unknown",
                message: format!(
                    "cannot determine appdata directory: {e} — set HOME or RUNNING_IN_CONTAINER=1"
                ),
            });
            return report.finish();
        }
    };

    if !data_dir.is_dir() {
        report.blocking_failures.push(SetupFailure {
            code: "appdata_missing",
            message: format!("appdata directory does not exist: {}", data_dir.display()),
        });
    }
    let env_path = data_dir.join(".env");
    if !env_path.is_file() {
        report.advisory_failures.push(SetupFailure {
            code: "env_file_missing",
            message: format!(
                "{} does not exist; setup repair will create one, but process env can supply values",
                env_path.display()
            ),
        });
    }
    if config.rarcane.api_url.is_empty() {
        report.blocking_failures.push(SetupFailure {
            code: "missing_rarcane_api_url",
            message: "RARCANE_API_URL is required".into(),
        });
    }
    if config.rarcane.api_key.is_empty() {
        report.blocking_failures.push(SetupFailure {
            code: "missing_rarcane_api_key",
            message: "RARCANE_API_KEY is required".into(),
        });
    }

    check_auth(config, &mut report);
    check_port(&config.mcp.host, config.mcp.port, &mut report);

    report.finish()
}

fn setup_repair(config: &Config) -> Result<SetupReport> {
    let data_dir = setup_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    write_env(&data_dir, config)?;

    // Re-run check after repair; `appdata_missing` is now resolved since
    // `create_dir_all` succeeded above.
    let mut report = setup_check(config, false);
    report.ran_repair = true;

    Ok(report.finish())
}

fn require_oauth_field(
    report: &mut SetupReport,
    value: &Option<String>,
    code: &'static str,
    message: &str,
) {
    if value.as_deref().unwrap_or("").is_empty() {
        report.blocking_failures.push(SetupFailure {
            code,
            message: message.into(),
        });
    }
}

fn check_auth(config: &Config, report: &mut SetupReport) {
    if let Err(error) = resolve_auth_policy_kind(config, config.mcp.trusted_gateway) {
        report.blocking_failures.push(SetupFailure {
            code: "invalid_auth_policy",
            message: error.to_string(),
        });
        return;
    }

    if config.mcp.no_auth {
        return;
    }

    if config.mcp.auth.mode == AuthMode::OAuth {
        require_oauth_field(
            report,
            &config.mcp.auth.public_url,
            "missing_oauth_public_url",
            "RARCANE_MCP_PUBLIC_URL is required for OAuth mode",
        );
        require_oauth_field(
            report,
            &config.mcp.auth.google_client_id,
            "missing_oauth_client_id",
            "RARCANE_MCP_GOOGLE_CLIENT_ID is required for OAuth mode",
        );
        require_oauth_field(
            report,
            &config.mcp.auth.google_client_secret,
            "missing_oauth_client_secret",
            "RARCANE_MCP_GOOGLE_CLIENT_SECRET is required for OAuth mode",
        );
        require_oauth_field(
            report,
            &Some(config.mcp.auth.admin_email.clone()),
            "missing_oauth_admin_email",
            "RARCANE_MCP_AUTH_ADMIN_EMAIL is required for OAuth mode",
        );
    } else if config.mcp.api_token.as_deref().unwrap_or("").is_empty() {
        report.blocking_failures.push(SetupFailure {
            code: "missing_mcp_token",
            message: "RARCANE_MCP_TOKEN is required unless no_auth or OAuth mode is enabled".into(),
        });
    }
}

fn check_port(host: &str, port: u16, report: &mut SetupReport) {
    if TcpListener::bind((host, port)).is_err() {
        report.advisory_failures.push(SetupFailure {
            code: "mcp_port_in_use",
            message: format!("MCP bind address {host}:{port} is already in use or unavailable"),
        });
    }
}

fn setup_data_dir() -> anyhow::Result<PathBuf> {
    // Writes go to the canonical service appdata dir — `~/.rarcane/` on bare
    // metal, `/data` in a container (see `default_data_dir`). This is the SAME
    // location the binary loads `.env` from at startup (`config::load_dotenv`),
    // so the plugin hook's writes and the server's reads always agree.
    //
    // An explicit `RARCANE_HOME` override is honored (used by tests).
    // `CLAUDE_PLUGIN_DATA` is intentionally NOT consulted: the plugin's sandboxed
    // data dir must not diverge from `~/.rarcane/`.
    if let Some(val) = std::env::var_os("RARCANE_HOME") {
        return Ok(PathBuf::from(val));
    }
    default_data_dir()
}

fn write_env(data_dir: &Path, config: &Config) -> Result<()> {
    let mut lines = vec![
        dotenv_assignment("RARCANE_API_URL", &config.rarcane.api_url)?,
        dotenv_assignment("RARCANE_API_KEY", &config.rarcane.api_key)?,
        dotenv_assignment("RARCANE_MCP_HOST", &config.mcp.host)?,
        dotenv_assignment("RARCANE_MCP_PORT", &config.mcp.port.to_string())?,
        dotenv_assignment("RARCANE_MCP_NO_AUTH", &config.mcp.no_auth.to_string())?,
    ];

    if let Some(token) = config.mcp.api_token.as_deref().filter(|v| !v.is_empty()) {
        lines.push(dotenv_assignment("RARCANE_MCP_TOKEN", token)?);
    }
    if config.mcp.auth.mode == AuthMode::OAuth {
        lines.push("RARCANE_MCP_AUTH_MODE=oauth".into());
        if let Some(v) = &config.mcp.auth.public_url {
            lines.push(dotenv_assignment("RARCANE_MCP_PUBLIC_URL", v)?);
        }
        if let Some(v) = &config.mcp.auth.google_client_id {
            lines.push(dotenv_assignment("RARCANE_MCP_GOOGLE_CLIENT_ID", v)?);
        }
        if let Some(v) = &config.mcp.auth.google_client_secret {
            lines.push(dotenv_assignment("RARCANE_MCP_GOOGLE_CLIENT_SECRET", v)?);
        }
        if !config.mcp.auth.admin_email.is_empty() {
            lines.push(dotenv_assignment(
                "RARCANE_MCP_AUTH_ADMIN_EMAIL",
                &config.mcp.auth.admin_email,
            )?);
        }
    }

    let env_path = data_dir.join(".env");
    let temp_path = unique_env_temp_path(data_dir);
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        // mode(0o600) sets permissions atomically at creation — no second chmod needed.
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temp_path)?;
        writeln!(file, "{}", lines.join("\n"))?;
        file.sync_all()?;
    }
    #[cfg(not(unix))]
    std::fs::write(&temp_path, format!("{}\n", lines.join("\n")))?;
    std::fs::rename(&temp_path, &env_path).inspect_err(|_| {
        // Best-effort cleanup: remove the temp file if rename fails to avoid leaking
        // a partially-written file containing secrets.
        let _ = std::fs::remove_file(&temp_path);
    })?;
    Ok(())
}

fn unique_env_temp_path(data_dir: &Path) -> PathBuf {
    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    data_dir.join(format!(".env.tmp.{}.{}", std::process::id(), sequence))
}

fn dotenv_assignment(key: &'static str, value: &str) -> Result<String> {
    Ok(format!("{key}={}", dotenv_value(value)?))
}

fn dotenv_value(value: &str) -> Result<String> {
    if value.chars().any(|c| matches!(c, '\n' | '\r' | '\0')) {
        bail!("dotenv values cannot contain newlines or NUL bytes");
    }

    if value.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '/' | ':' | '@' | '%' | '+' | '=' | ',')
    }) {
        return Ok(value.to_string());
    }

    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("\"{escaped}\""))
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
