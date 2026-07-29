use rarcane::{
    actions::ArcaneAction, mcp::execute_tool_without_peer_for_test, testing::loopback_state,
};
use serde_json::json;
use tokio::sync::Mutex;

static DESTRUCTIVE_ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn help_returns_json_object() {
    let state = loopback_state();
    let result = execute_tool_without_peer_for_test(&state, "arcane", json!({"action": "help"}))
        .await
        .expect("help should not require upstream Arcane");
    assert_eq!(result["tool"], "arcane");
    assert!(result["actions"].is_array());
}

#[tokio::test]
async fn status_returns_ok() {
    let state = loopback_state();
    let result = execute_tool_without_peer_for_test(&state, "arcane", json!({"action": "status"}))
        .await
        .expect("status should not require upstream Arcane");
    assert_eq!(result["status"], "ok");
}

#[tokio::test]
async fn destructive_action_requires_confirm_before_network() {
    let _guard = DESTRUCTIVE_ENV_LOCK.lock().await;
    let previous = std::env::var_os("RARCANE_MCP_ALLOW_DESTRUCTIVE");
    // SAFETY: edition 2024 marks set_var/remove_var unsafe because they race
    // with concurrent environment reads. DESTRUCTIVE_ENV_LOCK (held above)
    // serialises every test that touches this variable.
    unsafe { std::env::remove_var("RARCANE_MCP_ALLOW_DESTRUCTIVE") };
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(
        &state,
        "arcane",
        json!({
            "action": "container",
            "subaction": "stop",
            "envId": "env-1",
            "id": "ctr-1"
        }),
    )
    .await
    .expect_err("destructive action should be blocked");
    assert!(error.to_string().contains("confirmation required"));
    // SAFETY: still holding DESTRUCTIVE_ENV_LOCK; see above.
    unsafe {
        match previous {
            Some(value) => std::env::set_var("RARCANE_MCP_ALLOW_DESTRUCTIVE", value),
            None => std::env::remove_var("RARCANE_MCP_ALLOW_DESTRUCTIVE"),
        }
    }
}

#[test]
fn action_parses_arcane_tool_shape() {
    let action = ArcaneAction::from_mcp_args(&json!({
        "action": "volume",
        "subaction": "browse",
        "envId": "env-1",
        "id": "data",
        "params": {"path": "etc"}
    }))
    .expect("arcane tool args should parse");
    assert_eq!(action.action, "volume");
    assert_eq!(action.subaction.as_deref(), Some("browse"));
}

#[tokio::test]
async fn missing_action_is_rejected() {
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(&state, "arcane", json!({}))
        .await
        .expect_err("missing action should be rejected");
    assert!(error.to_string().contains("action is required"));
}
