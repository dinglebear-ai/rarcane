//! CLI — thin shim that parses args, calls `ArcaneService`, formats output.
//!
//! The CLI uses the same service layer as the MCP server. No business logic lives here.
//!
//! **Template**: add subcommands to match your service's operations.
//!
//! # Usage
//!
//! ```text
//! rarcane call --action container --subaction list --env-id env-abc
//! rarcane status
//! rarcane doctor [--json]
//! ```

use crate::{
    actions::ArcaneAction,
    app::{ArcaneService, local_help, local_status, validate_service_action},
    arcane::ArcaneClient,
    config::ArcaneConfig,
};
use anyhow::{Result, anyhow};

// TEMPLATE: The doctor module is the §48 reference implementation.
//           Import it from here and wire into run() below.
pub mod doctor;
pub mod setup;
pub mod watch;

pub use setup::{SetupCommand, apply_plugin_options, run_setup};

pub const USAGE: &str = "Usage:
  rarcane [serve]          Start MCP HTTP server (default)
  rarcane mcp              Start MCP stdio transport

  rarcane call --action ACTION [--subaction SUB] [--env-id ENV] [--id ID] [--params-json JSON] [--confirm]
  rarcane status                    Show local server status
  rarcane help [--domain DOMAIN]    Show JSON action reference
  rarcane doctor [--json]           Run environment pre-flight checks
  rarcane watch [--url URL] [--interval N]  Poll /health and emit on state change
  rarcane setup check               Check plugin setup without mutating appdata
  rarcane setup repair              Create missing appdata/env setup files
  rarcane setup plugin-hook [--no-repair]  Plugin hook JSON contract

  rarcane --help                    Show this help
  rarcane --version                 Show version

Environment:
  RARCANE_API_URL          Upstream service URL
  RARCANE_API_KEY          Upstream service API key
  RARCANE_MCP_HOST         Bind host (default 127.0.0.1)
  RARCANE_MCP_PORT         Bind port (default 40110)
  RARCANE_MCP_NO_AUTH      Disable auth (loopback only)
  RARCANE_MCP_TOKEN        Static bearer token
  RUST_LOG                 Log filter (e.g. info,rmcp=warn)";

pub fn usage() -> &'static str {
    USAGE
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Call {
        action: String,
        subaction: Option<String>,
        env_id: Option<String>,
        id: Option<String>,
        params: serde_json::Value,
    },
    Status,
    Help {
        domain: Option<String>,
    },
    /// Pre-flight environment validation (§48).
    ///
    /// TEMPLATE: Always keep this command. It is the operator's first stop
    /// when setting up or debugging the service.
    Doctor {
        /// Output JSON instead of human-readable text.
        json: bool,
    },
    /// Poll the MCP server health endpoint and emit a line on every state change.
    ///
    /// Designed to be run as a plugin monitor — stdout is the event stream,
    /// stderr is debug output. Exits only on CTRL+C.
    Watch {
        /// Base URL of the MCP server (default: http://localhost:{RARCANE_MCP_PORT}).
        url: Option<String>,
        /// Poll interval in seconds (default: 10).
        interval: u64,
    },
    Setup(SetupCommand),
}

/// Parse CLI arguments from `std::env::args()`.
///
/// Returns `None` if the first argument is not a known subcommand.
/// **Template**: extend this to use clap or another arg parser for a real CLI.
/// This is intentionally minimal so the template compiles without extra deps.
///
/// # TEMPLATE: Adding a new subcommand
///
/// 1. Add a variant to `Command` above.
/// 2. Add a match arm here to construct it from args.
/// 3. Add a dispatch arm in `run()` below.
/// 4. Update `USAGE` above.
pub fn parse_args() -> Result<Option<Command>> {
    parse_args_from(std::env::args().skip(1))
}

pub fn parse_args_from<I, S>(args: I) -> Result<Option<Command>>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let command = match args.as_slice() {
        [] => None,
        [subcommand, rest @ ..] => match subcommand.as_str() {
            "call" => Some(parse_call_flags(rest)?),
            "status" => {
                reject_args(rest, "status")?;
                Some(Command::Status)
            }
            "help" => Some(Command::Help {
                domain: parse_optional_value_flag(rest, "help", "--domain")?,
            }),
            // §48: doctor is always parsed here, dispatched via run_cli in main.rs.
            // TEMPLATE: Keep this arm. It routes to doctor::run_doctor() which needs
            //           the full Config (not just ArcaneConfig), so main.rs handles it.
            "doctor" => {
                let json = parse_bool_flag(rest, "doctor", "--json")?;
                Some(Command::Doctor { json })
            }
            "watch" => {
                let (url, interval_arg) = parse_watch_flags(rest)?;
                let interval = match interval_arg {
                    Some(v) => v.parse().map_err(|_| {
                        anyhow!("watch --interval must be a positive integer number of seconds")
                    })?,
                    None => 10,
                };
                if interval == 0 {
                    return Err(anyhow!(
                        "watch --interval must be a positive integer number of seconds"
                    ));
                }
                Some(Command::Watch { url, interval })
            }
            "setup" => match rest {
                [action, flags @ ..] if action == "check" => {
                    reject_args(flags, "setup check")?;
                    Some(Command::Setup(SetupCommand::Check))
                }
                [action, flags @ ..] if action == "repair" => {
                    reject_args(flags, "setup repair")?;
                    Some(Command::Setup(SetupCommand::Repair))
                }
                [action, flags @ ..] if action == "install" => {
                    reject_args(flags, "setup install")?;
                    Some(Command::Setup(SetupCommand::Install))
                }
                [action, flags @ ..] if action == "plugin-hook" => {
                    let no_repair = parse_bool_flag(flags, "setup plugin-hook", "--no-repair")?;
                    Some(Command::Setup(SetupCommand::PluginHook { no_repair }))
                }
                _ => None,
            },
            _ => None,
        },
    };
    Ok(command)
}

/// Run a CLI command, print the result, and exit.
///
/// # TEMPLATE
/// - `Doctor` is handled specially in `main.rs::run_cli` (needs full `Config`).
/// - All other commands get only `ArcaneConfig`; keep it that way.
/// - Add `--json` support to each new command by forwarding a `json` flag.
pub async fn run(cmd: Command, cfg: &ArcaneConfig) -> Result<()> {
    let result = match &cmd {
        Command::Status => local_status(),
        Command::Help { domain } => local_help(domain.as_deref()),
        Command::Call {
            action,
            subaction,
            env_id,
            id,
            params,
        } => {
            validate_service_action(action, subaction.as_deref())?;
            let service = ArcaneService::new(ArcaneClient::new(cfg)?);
            service
                .dispatch(&ArcaneAction {
                    action: action.clone(),
                    subaction: subaction.clone(),
                    env_id: env_id.clone(),
                    id: id.clone(),
                    params: params.clone(),
                })
                .await?
        }
        // Doctor, Watch, and Setup are never dispatched via this function — main.rs
        // handles them directly because they need config.mcp fields.
        Command::Doctor { .. } | Command::Watch { .. } | Command::Setup(_) => {
            unreachable!("dispatched directly in main.rs::run_cli")
        }
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

// ── arg parsing helpers ───────────────────────────────────────────────────────

fn reject_args(args: &[String], command: &str) -> Result<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{command} does not accept argument `{}`", args[0]))
    }
}

fn parse_bool_flag(args: &[String], command: &str, flag: &str) -> Result<bool> {
    let mut found = false;
    for arg in args {
        if arg == flag {
            if found {
                return Err(anyhow!("{command} received duplicate {flag}"));
            }
            found = true;
        } else {
            return Err(anyhow!("{command} does not accept argument `{arg}`"));
        }
    }
    Ok(found)
}

fn parse_optional_value_flag(args: &[String], command: &str, flag: &str) -> Result<Option<String>> {
    match args {
        [] => Ok(None),
        [found_flag, value] if found_flag == flag => {
            if value.starts_with("--") {
                Err(anyhow!("{command} requires a value after {flag}"))
            } else {
                Ok(Some(value.clone()))
            }
        }
        [found_flag] if found_flag == flag => {
            Err(anyhow!("{command} requires a value after {flag}"))
        }
        [found_flag, value, rest @ ..] if found_flag == flag => {
            if value.starts_with("--") {
                Err(anyhow!("{command} requires a value after {flag}"))
            } else if rest.iter().any(|arg| arg == flag) {
                Err(anyhow!("{command} received duplicate {flag}"))
            } else {
                Err(anyhow!("{command} does not accept argument `{}`", rest[0]))
            }
        }
        [unexpected, ..] => Err(anyhow!("{command} does not accept argument `{unexpected}`")),
    }
}

fn parse_call_flags(args: &[String]) -> Result<Command> {
    let mut action = None;
    let mut subaction = None;
    let mut env_id = None;
    let mut id = None;
    let mut params = serde_json::json!({});
    let mut confirm = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--action" => action = Some(required_next(args, &mut index, "--action")?),
            "--subaction" => subaction = Some(required_next(args, &mut index, "--subaction")?),
            "--env-id" => env_id = Some(required_next(args, &mut index, "--env-id")?),
            "--id" => id = Some(required_next(args, &mut index, "--id")?),
            "--params-json" => {
                let raw = required_next(args, &mut index, "--params-json")?;
                params = serde_json::from_str(&raw)
                    .map_err(|err| anyhow!("--params-json must be valid JSON object: {err}"))?;
                if !params.is_object() {
                    return Err(anyhow!("--params-json must be a JSON object"));
                }
            }
            "--confirm" => {
                confirm = true;
                index += 1;
            }
            other => return Err(anyhow!("call does not accept argument `{other}`")),
        }
    }

    if confirm {
        params
            .as_object_mut()
            .expect("params starts as object and parser enforces object")
            .insert("confirm".into(), serde_json::Value::Bool(true));
    }

    Ok(Command::Call {
        action: action.ok_or_else(|| anyhow!("call requires --action"))?,
        subaction,
        env_id,
        id,
        params,
    })
}

fn required_next(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    let Some(value) = args.get(*index + 1) else {
        return Err(anyhow!("call requires a value after {flag}"));
    };
    if value.starts_with("--") {
        return Err(anyhow!("call requires a value after {flag}"));
    }
    *index += 2;
    Ok(value.clone())
}

fn parse_watch_flags(args: &[String]) -> Result<(Option<String>, Option<String>)> {
    let mut url = None;
    let mut interval = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let target = match flag {
            "--url" => &mut url,
            "--interval" => &mut interval,
            _ => return Err(anyhow!("watch does not accept argument `{flag}`")),
        };
        if target.is_some() {
            return Err(anyhow!("watch received duplicate {flag}"));
        }
        let Some(value) = args.get(index + 1) else {
            return Err(anyhow!("watch requires a value after {flag}"));
        };
        if value.starts_with("--") {
            return Err(anyhow!("watch requires a value after {flag}"));
        }
        *target = Some(value.clone());
        index += 2;
    }
    Ok((url, interval))
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
