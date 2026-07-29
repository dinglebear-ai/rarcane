use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

use super::{
    MAX_CONCURRENT_HTTP_REQUESTS, MCP_BODY_LIMIT_BYTES, router, static_token_for_auth,
    with_mcp_concurrency_limit,
};

#[tokio::test]
async fn health_is_served_without_auth() {
    let response = router(crate::testing::loopback_state())
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn mcp_rejects_oversized_request_bodies() {
    let response = router(crate::testing::loopback_state())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CONTENT_LENGTH, MCP_BODY_LIMIT_BYTES + 1)
                .body(Body::from(vec![b'x'; MCP_BODY_LIMIT_BYTES + 1]))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn bearer_auth_rejects_missing_and_invalid_tokens() {
    for authorization in [None, Some("Bearer wrong")] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(value) = authorization {
            request = request.header(header::AUTHORIZATION, value);
        }
        let response = router(crate::testing::bearer_state("secret"))
            .oneshot(
                request
                    .body(Body::from("{}"))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn bearer_auth_accepts_the_configured_token() {
    let response = router(crate::testing::bearer_state("secret"))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer secret")
                .body(Body::from("{}"))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn mcp_concurrency_is_bounded() {
    let limit = std::hint::black_box(MAX_CONCURRENT_HTTP_REQUESTS);
    assert!(limit > 0);
    assert!(limit <= 32);
}

#[test]
fn oauth_can_disable_or_retain_the_static_token() {
    let mut config = crate::config::McpConfig {
        api_token: Some("migration-token".into()),
        auth: crate::config::AuthConfig {
            mode: crate::config::AuthMode::OAuth,
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        static_token_for_auth(&config).as_deref(),
        Some("migration-token"),
        "OAuth and bearer should coexist by default"
    );
    config.auth.disable_static_token_with_oauth = true;
    assert!(
        static_token_for_auth(&config).is_none(),
        "OAuth-only mode must not mount the static bearer token"
    );
}

#[tokio::test]
async fn mcp_concurrency_does_not_block_public_or_oauth_routes() {
    let constrained_mcp = with_mcp_concurrency_limit(
        axum::Router::new().route(
            "/mcp",
            axum::routing::get(|| async {
                std::future::pending::<()>().await;
                "unreachable"
            }),
        ),
        1,
    );
    let app = constrained_mcp
        .route("/health", axum::routing::get(super::health))
        .route("/status", axum::routing::get(|| async { "status" }))
        .route("/oauth/authorize", axum::routing::get(|| async { "oauth" }));

    let blocked = tokio::spawn(
        app.clone().oneshot(
            Request::builder()
                .uri("/mcp")
                .body(Body::empty())
                .expect("MCP request should build"),
        ),
    );
    tokio::task::yield_now().await;

    for path in ["/health", "/status", "/oauth/authorize"] {
        let response = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            app.clone().oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("public request should build"),
            ),
        )
        .await
        .unwrap_or_else(|_| panic!("{path} must not queue behind saturated MCP traffic"))
        .unwrap_or_else(|error| panic!("{path} should respond: {error}"));
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
    blocked.abort();
}

#[tokio::test]
async fn oauth_token_enforces_read_and_write_action_scopes() {
    let directory = tempfile::tempdir().expect("temporary auth directory");
    let state = crate::testing::oauth_state(directory.path()).await;
    let auth_state = match &state.auth_policy {
        crate::server::AuthPolicy::Mounted {
            auth_state: Some(auth_state),
        } => auth_state,
        _ => panic!("OAuth test state should mount auth"),
    };
    let now = lab_auth::util::now_unix() as usize;
    let token = auth_state
        .signing_keys
        .issue_access_token(&lab_auth::jwt::AccessClaims {
            iss: "https://rarcane.rarcane.com".into(),
            sub: "reader@example.com".into(),
            aud: lab_auth::metadata::canonical_resource_url(auth_state),
            exp: now + 60,
            iat: now,
            jti: "route-test".into(),
            scope: crate::actions::READ_SCOPE.into(),
            azp: String::new(),
        })
        .expect("token should issue");

    let read = mcp_call(
        state.clone(),
        &token,
        serde_json::json!({
            "action": "status"
        }),
    )
    .await;
    assert_eq!(read.status(), StatusCode::OK);
    let read_body = axum::body::to_bytes(read.into_body(), 64 * 1024)
        .await
        .expect("read response body");
    let read_text = String::from_utf8_lossy(&read_body);
    assert!(
        read_text.contains(r#"\"status\":\"ok\""#),
        "read-scoped token should execute the status action: {read_text}"
    );
    assert!(
        !read_text.contains("forbidden") && !read_text.contains("requires scope"),
        "read-scoped token was incorrectly forbidden: {read_text}"
    );

    let write = mcp_call(
        state,
        &token,
        serde_json::json!({
            "action": "container",
            "subaction": "delete",
            "envId": "test",
            "id": "container",
            "params": {"confirm": true}
        }),
    )
    .await;
    assert_eq!(write.status(), StatusCode::OK);
    let write_body = axum::body::to_bytes(write.into_body(), 64 * 1024)
        .await
        .expect("write response body");
    assert!(
        String::from_utf8_lossy(&write_body).contains("requires scope: rarcane:write"),
        "unexpected write response: {}",
        String::from_utf8_lossy(&write_body)
    );
}

async fn mcp_call(
    state: crate::server::AppState,
    token: &str,
    arguments: serde_json::Value,
) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::HOST, "rarcane.rarcane.com")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json, text/event-stream")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": {"name": "arcane", "arguments": arguments}
                    }))
                    .expect("request should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond")
}
