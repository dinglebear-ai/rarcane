//! Binary entry point — mode dispatch only.
//!
//! Modes:
//!   `rarcane [serve]`        Start MCP HTTP server (default if no args)
//!   `rarcane mcp`            Start MCP stdio transport
//!   `rarcane greet ...`      CLI greet command
//!   `rarcane echo ...`       CLI echo command
//!   `rarcane status`         CLI status command
//!   `rarcane --help`         Print usage
//!   `rarcane --version`      Print version
//!
//! **Template**: add your binary name in Cargo.toml `[[bin]] name = "..."`.
//! Extend `run_cli` if you add more CLI subcommands.

use anyhow::Result;
use std::sync::Arc;

use rarcane::{
    app::ArcaneService,
    arcane::ArcaneClient,
    cli,
    config::Config,
    mcp,
    server::{self, AppState, AuthPolicy, AuthPolicyKind, resolve_auth_policy_kind},
};
use rmcp::{ServiceExt, transport::stdio};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Handle meta-flags before initialising logging (they print and exit)
    match args.as_slice() {
        [f] if matches!(f.as_str(), "--help" | "-h") => {
            eprintln!("{}", cli::usage());
            return Ok(());
        }
        [f] if matches!(f.as_str(), "--version" | "-V" | "version") => {
            println!("rarcane {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    // Load ~/.rarcane/.env (or /data/.env in a container) before any Config::load
    // so the binary works on bare metal without a process manager injecting env.
    // Non-overriding: explicit process env still wins.
    rarcane::config::load_dotenv()?;

    // Suppress logs in stdio/CLI mode — MCP clients communicate over stdio
    // and cannot tolerate log lines mixed into the JSON stream.
    let stdio_mode = matches!(args.as_slice(), [c] if c == "mcp");
    let serve_mode = args.is_empty()
        || matches!(args.as_slice(), [c] if c == "serve")
        || matches!(args.as_slice(), [a, b] if a == "serve" && b == "mcp");

    let log_level = if stdio_mode || !serve_mode {
        "warn"
    } else {
        "info"
    };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();

    if serve_mode {
        serve_mcp().await
    } else if stdio_mode {
        serve_stdio_mcp().await
    } else {
        run_cli().await
    }
}

// ── modes ─────────────────────────────────────────────────────────────────────

/// Start the MCP HTTP server (Streamable HTTP transport).
async fn serve_mcp() -> Result<()> {
    let config = Config::load()?;
    let state = build_state(config).await?;

    info!(
        bind = %state.config.bind_addr(),
        server_name = %state.config.server_name,
        auth = ?state.auth_policy,
        "MCP HTTP server starting"
    );

    let bind = state.config.bind_addr();
    let app = server::router(state).layer(tower_http::trace::TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(bind = %bind, "MCP HTTP server listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Start the MCP stdio transport (for local/subprocess MCP clients).
///
/// Stdio is always LoopbackDev — it's a local trusted pipe between parent and
/// child process. HTTP auth middleware doesn't apply; forcing Mounted here
/// breaks all stdio clients with "forbidden: missing http context".
async fn serve_stdio_mcp() -> Result<()> {
    let config = Config::load()?;
    let service = ArcaneService::new(ArcaneClient::new(&config.rarcane)?);
    let state = AppState {
        config: config.mcp,
        auth_policy: AuthPolicy::LoopbackDev, // stdio = trusted local transport
        service,
    };
    let svc = mcp::rmcp_server(state).serve(stdio()).await?;
    svc.waiting().await?;
    Ok(())
}

/// Dispatch CLI subcommands.
async fn run_cli() -> Result<()> {
    let parsed = cli::parse_args()?;
    // Translate CLAUDE_PLUGIN_OPTION_* into RARCANE_* env vars BEFORE Config::load()
    // so `setup plugin-hook` sees plugin settings. rarcane is template-style:
    // setup_check validates the pre-loaded &Config.
    if matches!(
        parsed,
        Some(cli::Command::Setup(cli::SetupCommand::PluginHook { .. }))
    ) {
        cli::apply_plugin_options();
    }
    let config = Config::load()?;
    match parsed {
        Some(cli::Command::Doctor { json }) => {
            // Doctor needs the full Config (not just ArcaneConfig) to check
            // MCP port, auth mode, etc. — intercept here before service construction.
            cli::doctor::run_doctor(&config, json).await
        }
        Some(cli::Command::Watch { url, interval }) => {
            // Watch needs the MCP port to build the default URL but no service layer.
            let base = url.unwrap_or_else(|| format!("http://localhost:{}", config.mcp.port));
            cli::watch::run_watch(&base, interval).await
        }
        Some(cli::Command::Setup(command)) => cli::run_setup(&config, command).await,
        Some(cmd) => cli::run(cmd, &config.rarcane).await,
        None => {
            eprintln!("Unknown command. Run `rarcane --help` for usage.");
            std::process::exit(1);
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

async fn build_state(config: Config) -> Result<AppState> {
    let auth_policy = build_auth_policy(&config).await?;
    let service = ArcaneService::new(ArcaneClient::new(&config.rarcane)?);
    Ok(AppState {
        config: config.mcp,
        auth_policy,
        service,
    })
}

async fn build_auth_policy(config: &Config) -> Result<AuthPolicy> {
    match resolve_auth_policy_kind(config, config.mcp.trusted_gateway)? {
        AuthPolicyKind::LoopbackDev => Ok(AuthPolicy::LoopbackDev),
        AuthPolicyKind::TrustedGatewayUnscoped => Ok(AuthPolicy::TrustedGatewayUnscoped),
        AuthPolicyKind::MountedBearer => Ok(AuthPolicy::Mounted { auth_state: None }),
        AuthPolicyKind::MountedOAuth => {
            let auth_cfg = lab_auth::config::AuthConfigBuilder::new()
                .env_prefix("RARCANE_MCP")
                .session_cookie_name("example_mcp_session")
                .scopes_supported(vec![
                    rarcane::actions::READ_SCOPE.into(),
                    rarcane::actions::WRITE_SCOPE.into(),
                ])
                .default_scope("rarcane:read")
                .resource_path("/mcp")
                .enable_dynamic_registration(true)
                .build_from_sources(std::env::vars())
                .map_err(|e| anyhow::anyhow!("OAuth config error: {e}"))?;
            let auth_state = lab_auth::state::AuthState::new(auth_cfg)
                .await
                .map_err(|e| anyhow::anyhow!("OAuth state init error: {e}"))?;
            Ok(AuthPolicy::Mounted {
                auth_state: Some(Arc::new(auth_state)),
            })
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "CTRL+C handler failed");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "SIGTERM handler failed");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown signal received");
}
