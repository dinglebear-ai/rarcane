//! `rarcane` library crate.
//!
//! Exposes the service layer, config, and transport client so that integration
//! tests can import them without duplicating state construction.
//!
//! Public modules:
//!   [`app`]     — `ArcaneService` (business logic)
//!   [`config`]  — `Config`, `ArcaneConfig`, `McpConfig`
//!   `arcane` — `ArcaneClient` (bounded Arcane REST transport)
//!   [`mcp`]     — MCP protocol layer (tools, schemas, prompts, server handler)
//!   [`server`]  — `AppState`, `AuthPolicy`, HTTP router

pub mod actions;
pub mod app;
pub mod arcane;
pub mod cli;
pub mod config;
pub mod logging;
pub mod mcp;
pub mod server;
pub mod token_limit;

/// Test helpers — available when `features = ["test-support"]` or in `cfg(test)`.
///
/// Use these in integration tests to construct `AppState` without real creds.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod testing {
    use std::sync::Arc;

    use crate::{
        app::ArcaneService,
        arcane::ArcaneClient,
        config::{ArcaneConfig, McpConfig},
        server::{AppState, AuthPolicy},
    };

    fn stub_service() -> ArcaneService {
        let client = ArcaneClient::new(&ArcaneConfig {
            api_url: "http://localhost:1/stub".into(),
            api_key: "test".into(),
        })
        .expect("stub client should always build");
        ArcaneService::new(client)
    }

    /// `AppState` with no auth (loopback trust boundary).
    /// Use this for unit tests that don't need auth.
    pub fn loopback_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service(),
        }
    }

    /// `AppState` requiring a static bearer token.
    pub fn bearer_state(token: &str) -> AppState {
        AppState {
            config: McpConfig {
                api_token: Some(token.to_string()),
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted { auth_state: None },
            service: stub_service(),
        }
    }

    /// `AppState` with full OAuth (requires data directory for SQLite + key file).
    pub async fn oauth_state(data_dir: &std::path::Path) -> AppState {
        let auth_state = build_auth_state(data_dir).await;
        AppState {
            config: McpConfig {
                auth: crate::config::AuthConfig {
                    public_url: Some("https://rarcane.rarcane.com".to_string()),
                    ..Default::default()
                },
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted {
                auth_state: Some(Arc::new(auth_state)),
            },
            service: stub_service(),
        }
    }

    pub async fn build_auth_state(data_dir: &std::path::Path) -> lab_auth::state::AuthState {
        let vars: Vec<(String, String)> = vec![
            ("RARCANE_MCP_AUTH_MODE".into(), "oauth".into()),
            (
                "RARCANE_MCP_PUBLIC_URL".into(),
                "https://rarcane.rarcane.com".into(),
            ),
            (
                "RARCANE_MCP_GOOGLE_CLIENT_ID".into(),
                "test-client-id".into(),
            ),
            (
                "RARCANE_MCP_GOOGLE_CLIENT_SECRET".into(),
                "test-client-secret".into(),
            ),
            (
                "RARCANE_MCP_AUTH_ADMIN_EMAIL".into(),
                "admin@rarcane.com".into(),
            ),
            (
                "RARCANE_MCP_AUTH_SQLITE_PATH".into(),
                data_dir.join("auth.db").display().to_string(),
            ),
            (
                "RARCANE_MCP_AUTH_KEY_PATH".into(),
                data_dir.join("auth-jwt.pem").display().to_string(),
            ),
        ];

        let auth_config = lab_auth::config::AuthConfigBuilder::new()
            .env_prefix("RARCANE_MCP")
            .session_cookie_name("example_mcp_session")
            .scopes_supported(vec![
                crate::actions::READ_SCOPE.into(),
                crate::actions::WRITE_SCOPE.into(),
            ])
            .default_scope("rarcane:read")
            .resource_path("/mcp")
            .build_from_sources(vars)
            .expect("test auth config should build");

        lab_auth::state::AuthState::new(auth_config)
            .await
            .expect("test auth state should init")
    }
}
