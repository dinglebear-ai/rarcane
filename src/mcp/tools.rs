//! MCP tool dispatch — thin shims only.
//!
//! Parse JSON args, call `ArcaneService`, return JSON. Business validation,
//! endpoint selection, and safety gates live in `app.rs`.

use rmcp::{
    RoleServer,
    service::{ElicitationError, Peer},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::{ArcaneAction, execute_service_action};
use crate::app::{ElicitedNameOutcome, ScaffoldIntent};
use crate::server::AppState;

pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
    peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    match name {
        "arcane" => dispatch_arcane(state, args, peer).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub async fn execute_tool_without_peer_for_test(
    state: &AppState,
    name: &str,
    args: Value,
) -> anyhow::Result<Value> {
    match name {
        "arcane" => {
            let action = ArcaneAction::from_mcp_args(&args)?;
            if matches!(action.action.as_str(), "elicit_name" | "scaffold_intent") {
                return Err(anyhow::anyhow!(
                    "action={} requires an MCP peer",
                    action.action
                ));
            }
            execute_service_action(&state.service, &action).await
        }
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

async fn dispatch_arcane(
    state: &AppState,
    args: Value,
    peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    let action = ArcaneAction::from_mcp_args(&args)?;
    match action.action.as_str() {
        "elicit_name" => elicit_name(state, peer).await,
        "scaffold_intent" => elicit_scaffold_intent(state, peer).await,
        _ => execute_service_action(&state.service, &action).await,
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct NameInput {
    /// Your first name, or whatever you would like to be called.
    name: String,
}

rmcp::elicit_safe!(NameInput);

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ScaffoldIntentInput {
    display_name: String,
    crate_name: String,
    binary_name: String,
    server_category: String,
    env_prefix: String,
    auth_kind: String,
    host: String,
    port: u16,
    mcp_transport: String,
    mcp_primitives: String,
    deployment: String,
    plugins: String,
    publish_mcp: bool,
    crawl_urls: String,
    crawl_repos: String,
    crawl_search_topics: String,
}

rmcp::elicit_safe!(ScaffoldIntentInput);

impl From<ScaffoldIntentInput> for ScaffoldIntent {
    fn from(input: ScaffoldIntentInput) -> Self {
        Self {
            display_name: input.display_name,
            crate_name: input.crate_name,
            binary_name: input.binary_name,
            server_category: input.server_category,
            env_prefix: input.env_prefix,
            auth_kind: input.auth_kind,
            host: input.host,
            port: input.port,
            mcp_transport: input.mcp_transport,
            mcp_primitives: input.mcp_primitives,
            deployment: input.deployment,
            plugins: input.plugins,
            publish_mcp: input.publish_mcp,
            crawl_urls: input.crawl_urls,
            crawl_repos: input.crawl_repos,
            crawl_search_topics: input.crawl_search_topics,
        }
    }
}

async fn elicit_name(state: &AppState, peer: &Peer<RoleServer>) -> anyhow::Result<Value> {
    let response = peer.elicit::<NameInput>("What is your name?").await;
    let value = match response {
        Ok(Some(input)) => state
            .service
            .elicited_name_greeting(ElicitedNameOutcome::Accepted(&input.name)),
        Ok(None) => state
            .service
            .elicited_name_greeting(ElicitedNameOutcome::NoInput),
        Err(ElicitationError::UserDeclined) => state
            .service
            .elicited_name_greeting(ElicitedNameOutcome::Declined),
        Err(ElicitationError::UserCancelled) => state
            .service
            .elicited_name_greeting(ElicitedNameOutcome::Cancelled),
        Err(ElicitationError::CapabilityNotSupported) => state
            .service
            .elicited_name_greeting(ElicitedNameOutcome::Unsupported),
        Err(error) => return Err(anyhow::anyhow!("elicitation failed: {error}")),
    };
    Ok(value)
}

async fn elicit_scaffold_intent(
    state: &AppState,
    peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    match peer
        .elicit::<ScaffoldIntentInput>(
            "Describe the MCP server to scaffold. This returns intent JSON and does not mutate files.",
        )
        .await
    {
        Ok(Some(input)) => state.service.scaffold_intent(input.into()),
        Ok(None) => Ok(scaffold_status("no_input", "No scaffold intent was provided.")),
        Err(ElicitationError::UserDeclined) => Ok(scaffold_status(
            "declined",
            "User declined to provide scaffold intent.",
        )),
        Err(ElicitationError::UserCancelled) => Ok(scaffold_status(
            "cancelled",
            "Scaffold intent elicitation was cancelled.",
        )),
        Err(ElicitationError::CapabilityNotSupported) => Ok(scaffold_status(
            "elicitation_not_supported",
            "This MCP client does not support elicitation; use the scaffold-project skill manually.",
        )),
        Err(error) => Err(anyhow::anyhow!(
            "scaffold intent elicitation failed: {error}"
        )),
    }
}

fn scaffold_status(status: &str, message: &str) -> Value {
    json!({
        "kind": "rarcane_scaffold_intent",
        "schema_version": 1,
        "status": status,
        "message": message,
    })
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
