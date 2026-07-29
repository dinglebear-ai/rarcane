use super::*;
use crate::{
    actions::{ACTION_SPECS, ArcaneAction},
    arcane::ArcaneClient,
    config::ArcaneConfig,
};
use serde_json::json;
use tokio::sync::Mutex;

static DESTRUCTIVE_ENV_LOCK: Mutex<()> = Mutex::const_new(());

fn stub_service() -> ArcaneService {
    let client = ArcaneClient::new(&ArcaneConfig {
        api_url: "http://localhost:1".to_string(),
        api_key: "test-key".to_string(),
    })
    .expect("stub client should build");
    ArcaneService::new(client)
}

#[tokio::test]
async fn status_returns_local_ok() {
    let result = stub_service().status().await.expect("status should work");
    assert_eq!(result["status"], "ok");
    assert_eq!(result["upstream"], "arcane");
}

#[tokio::test]
async fn help_does_not_call_upstream() {
    let result = stub_service()
        .dispatch(&ArcaneAction {
            action: "help".into(),
            subaction: Some("container".into()),
            env_id: None,
            id: None,
            params: json!({}),
        })
        .await
        .expect("help should work without upstream");
    assert_eq!(result["tool"], "arcane");
    assert!(result["actions"].is_array());
}

#[tokio::test]
async fn generic_service_dispatch_rejects_mcp_only_actions() {
    let error = stub_service()
        .dispatch(&ArcaneAction {
            action: "elicit_name".into(),
            subaction: None,
            env_id: None,
            id: None,
            params: json!({}),
        })
        .await
        .expect_err("generic service dispatch must not send MCP-only actions upstream");
    assert!(error.to_string().contains("MCP-only"), "{error}");
}

#[tokio::test]
async fn destructive_actions_require_boolean_confirm() {
    let _guard = DESTRUCTIVE_ENV_LOCK.lock().await;
    let previous = std::env::var_os("RARCANE_MCP_ALLOW_DESTRUCTIVE");
    // SAFETY: edition 2024 marks set_var/remove_var unsafe because they race
    // with concurrent environment reads. DESTRUCTIVE_ENV_LOCK (held above)
    // serialises every test that touches this variable.
    unsafe { std::env::remove_var("RARCANE_MCP_ALLOW_DESTRUCTIVE") };
    let error = stub_service()
        .dispatch(&ArcaneAction {
            action: "project".into(),
            subaction: Some("down".into()),
            env_id: Some("env-1".into()),
            id: Some("stack".into()),
            params: json!({}),
        })
        .await
        .expect_err("destructive action should be blocked before network");
    assert!(error.to_string().contains("confirmation required"));
    // SAFETY: still holding DESTRUCTIVE_ENV_LOCK; see above.
    unsafe {
        match previous {
            Some(value) => std::env::set_var("RARCANE_MCP_ALLOW_DESTRUCTIVE", value),
            None => std::env::remove_var("RARCANE_MCP_ALLOW_DESTRUCTIVE"),
        }
    }
}

#[tokio::test]
async fn browse_rejects_path_traversal_before_network() {
    let error = stub_service()
        .dispatch(&ArcaneAction {
            action: "volume".into(),
            subaction: Some("browse".into()),
            env_id: Some("env-1".into()),
            id: Some("data".into()),
            params: json!({"path": "../secret"}),
        })
        .await
        .expect_err("bad path should be blocked");
    assert!(error.to_string().contains("relative path"));
}

#[tokio::test]
async fn restore_rejects_missing_backup_id_before_network() {
    let error = stub_service()
        .dispatch(&ArcaneAction {
            action: "volume".into(),
            subaction: Some("restore".into()),
            env_id: Some("env-1".into()),
            id: Some("volume-1".into()),
            params: json!({"confirm": true}),
        })
        .await
        .expect_err("backupId should be required");
    assert!(error.to_string().contains("backupId"));
}

#[test]
fn pagination_defaults_and_limits_are_enforced() {
    let spec = spec_for("container", Some("list")).expect("list spec");
    let action = ArcaneAction {
        action: "container".into(),
        subaction: Some("list".into()),
        env_id: Some("env-1".into()),
        id: None,
        params: json!({}),
    };
    assert_eq!(
        query_params(spec, &action).unwrap(),
        Some(json!({"offset": 0, "limit": 50}))
    );

    let excessive = ArcaneAction {
        params: json!({"limit": 201}),
        ..action
    };
    assert!(query_params(spec, &excessive).is_err());
}

#[test]
fn every_registry_path_can_be_built_without_placeholders() {
    for spec in ACTION_SPECS.iter().filter(|spec| !spec.path.is_empty()) {
        let action = ArcaneAction {
            action: spec.action.into(),
            subaction: spec.subaction.map(str::to_owned),
            env_id: Some("env id".into()),
            id: Some("resource/id".into()),
            params: json!({"backupId": "backup/id", "imageRef": "repo/image:tag", "confirm": true}),
        };
        validate_request(spec, &action).unwrap_or_else(|error| panic!("{}: {error}", spec.key()));
        let path = build_path(spec.path, &action).unwrap();
        assert!(
            !path.contains('{'),
            "{} left placeholder in {path}",
            spec.key()
        );
        reqwest::Method::from_bytes(spec.method.as_bytes())
            .unwrap_or_else(|error| panic!("{} has invalid method: {error}", spec.key()));
        let query = query_params(spec, &action)
            .unwrap_or_else(|error| panic!("{} query failed: {error}", spec.key()));
        if spec.method != "GET" {
            assert!(query.is_none(), "{} sent a query on a write", spec.key());
        }
        let body = body_params(spec.body, &action.params);
        assert_eq!(
            body.is_some(),
            spec.body != crate::actions::BodyMode::None,
            "{} body contract drifted",
            spec.key()
        );
    }
}

#[test]
fn scaffold_intent_transformation_lives_in_service() {
    let result = stub_service()
        .scaffold_intent(ScaffoldIntent {
            display_name: "Lab Gateway".into(),
            crate_name: "lab-gateway-mcp".into(),
            binary_name: "lab-gateway".into(),
            server_category: "application platform".into(),
            env_prefix: "lab".into(),
            auth_kind: "api key".into(),
            host: "".into(),
            port: 3100,
            mcp_transport: "streamable-http".into(),
            mcp_primitives: "tools, resources, tools".into(),
            deployment: "containers".into(),
            plugins: "claude, gemini, none".into(),
            publish_mcp: true,
            crawl_urls: "https://docs.rarcane.test".into(),
            crawl_repos: "".into(),
            crawl_search_topics: "Lab API".into(),
        })
        .expect("valid scaffold intent should build");

    assert_eq!(result["kind"], "rarcane_scaffold_intent");
    assert_eq!(result["server_category"], "application-platform");
    assert_eq!(result["project"]["env_prefix"], "LAB");
    assert_eq!(
        result["required_surfaces"],
        json!(["api", "cli", "mcp", "web"])
    );
}
