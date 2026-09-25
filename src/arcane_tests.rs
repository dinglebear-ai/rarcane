use super::*;
use crate::config::ArcaneConfig;
use std::io::{Read, Write};

#[test]
fn client_requires_arcane_config() {
    let result = ArcaneClient::new(&ArcaneConfig::default());
    assert!(result.is_err());
    let message = result
        .err()
        .expect("missing config should error")
        .to_string();
    assert!(message.contains("RARCANE_API_URL"));
}

#[test]
fn base_url_normalizes_api_suffix() {
    assert_eq!(
        normalize_base_url("https://arcane.test"),
        "https://arcane.test/api"
    );
    assert_eq!(
        normalize_base_url("https://arcane.test/api/"),
        "https://arcane.test/api"
    );
}

#[test]
fn path_segments_are_percent_encoded() {
    assert_eq!(encode_path_segment("abc/def"), "abc%2Fdef");
    assert_eq!(encode_path_segment("nginx:latest"), "nginx%3Alatest");
}

#[test]
fn client_error_kinds_are_structured() {
    let error = ArcaneError::Http {
        status: reqwest::StatusCode::BAD_GATEWAY,
        message: "upstream unavailable".into(),
    };
    assert_eq!(error.status(), Some(reqwest::StatusCode::BAD_GATEWAY));
    assert!(!error.is_retryable() || error.status().unwrap().is_server_error());
}

#[test]
fn upstream_response_limit_is_bounded() {
    assert!(std::hint::black_box(MAX_UPSTREAM_RESPONSE_BYTES) <= 8 * 1024 * 1024);
}

#[test]
fn upstream_error_redaction_removes_header_name_and_api_key() {
    let message = "authentication failed: X-API-Key secret-value is invalid";
    let redacted = redact(message, "secret-value");

    assert!(!redacted.contains("X-API-Key"));
    assert!(!redacted.contains("secret-value"));
    assert!(redacted.contains("[redacted-header]"));
    assert!(redacted.contains("[redacted-secret]"));
}

#[tokio::test]
async fn rejects_an_upstream_response_declared_over_the_limit() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    let address = listener.local_addr().expect("listener has an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("request should arrive");
        let mut request = [0_u8; 2048];
        let _ = stream
            .read(&mut request)
            .expect("request should be readable");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            MAX_UPSTREAM_RESPONSE_BYTES + 1
        )
        .expect("response should write");
        std::thread::sleep(std::time::Duration::from_millis(100));
    });
    let client = ArcaneClient::new(&ArcaneConfig {
        api_url: format!("http://{address}"),
        api_key: "test".into(),
    })
    .expect("client should build");

    let error = client
        .request(Method::GET, "/status", None, None, None)
        .await
        .expect_err("oversized response should fail closed");
    assert!(
        matches!(error, ArcaneError::ResponseTooLarge { .. }),
        "unexpected error: {error:?}"
    );
    server.join().expect("server thread should finish");
}
