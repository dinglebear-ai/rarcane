use serde_json::Value;
use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|err| panic!("failed to parse {path}: {err}"))
}

#[test]
fn plugin_manifests_exist_for_all_supported_hosts() {
    for path in [
        "plugins/rarcane/.claude-plugin/plugin.json",
        "plugins/rarcane/.codex-plugin/plugin.json",
        "plugins/rarcane/gemini-extension.json",
        "plugins/rarcane/.mcp.json",
        "plugins/rarcane/skills/rarcane/SKILL.md",
    ] {
        assert!(std::path::Path::new(path).exists(), "{path} should exist");
    }
}

#[test]
fn plugin_manifests_share_identity_and_connection_settings() {
    let claude = json("plugins/rarcane/.claude-plugin/plugin.json");
    let codex = json("plugins/rarcane/.codex-plugin/plugin.json");
    let gemini = json("plugins/rarcane/gemini-extension.json");
    let mcp = json("plugins/rarcane/.mcp.json");

    assert_eq!(claude["name"], "rarcane");
    assert_eq!(codex["name"], "rarcane-mcp");
    assert_eq!(gemini["name"], "rarcane-mcp");

    assert!(claude["repository"].as_str().unwrap().ends_with("/rarcane"));
    assert!(codex["repository"].as_str().unwrap().ends_with("/rarcane"));
    assert!(gemini["repository"].as_str().unwrap().ends_with("/rarcane"));

    let user_config = claude["userConfig"].as_object().unwrap();
    for key in [
        "server_url",
        "api_token",
        "rarcane_api_url",
        "rarcane_api_key",
    ] {
        assert!(
            user_config.contains_key(key),
            "Claude userConfig missing {key}"
        );
    }

    let gemini_settings: Vec<&str> = gemini["settings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|setting| setting["name"].as_str().unwrap())
        .collect();
    for key in [
        "server_url",
        "api_token",
        "rarcane_api_url",
        "rarcane_api_key",
    ] {
        assert!(
            gemini_settings.contains(&key),
            "Gemini settings missing {key}"
        );
    }

    assert_eq!(mcp["mcpServers"]["rarcane"]["command"], "npx");
    assert_eq!(
        mcp["mcpServers"]["rarcane"]["args"],
        serde_json::json!(["-y", "@dinglebear/rarcane", "mcp"])
    );
    assert_eq!(
        gemini["mcpServers"]["rarcane"]["url"],
        "${settings.server_url}/mcp"
    );
    assert_eq!(
        gemini["mcpServers"]["rarcane"]["headers"]["Authorization"],
        "Bearer ${settings.api_token}"
    );
}

#[test]
fn plugin_ships_no_lifecycle_hooks() {
    assert!(
        !std::path::Path::new("plugins/rarcane/hooks").exists(),
        "plugins/rarcane/hooks was retired; do not reintroduce plugin lifecycle hooks"
    );

    for path in [
        "plugins/rarcane/.claude-plugin/plugin.json",
        "plugins/rarcane/gemini-extension.json",
    ] {
        assert!(
            json(path).get("hooks").is_none(),
            "{path} must not declare a `hooks` key"
        );
    }
}

#[test]
fn plugin_hook_standard_is_documented() {
    let plugins = read("docs/PLUGINS.md");
    let patterns = read("docs/PATTERNS.md");
    for doc in [plugins, patterns] {
        assert!(doc.contains("<binary> setup plugin-hook"));
        assert!(doc.contains("<binary> setup plugin-hook --no-repair"));
        assert!(doc.contains("exit_policy"));
        assert!(doc.contains("blocking_failures"));
        assert!(doc.contains("advisory_failures"));
        assert!(doc.contains("ran_repair"));
    }
}

fn example_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rarcane")
}

fn setup_command(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(example_bin());
    cmd.env_clear()
        .current_dir(isolated_command_cwd(data_dir))
        .env("HOME", data_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("RARCANE_HOME", data_dir)
        .env("RARCANE_API_URL", "https://api.rarcane.test")
        .env("RARCANE_API_KEY", "rarcane-secret")
        .env("RARCANE_MCP_PORT", "0")
        .env("RARCANE_MCP_TOKEN", "mcp-secret");
    cmd
}

/// Run subprocess checks outside the repository so an ignored developer
/// `config.toml` cannot leak into tests through Config's local-dev fallback.
/// For repair tests the data directory may not exist yet, so use its existing
/// temporary parent without creating the directory under test.
fn isolated_command_cwd(data_dir: &std::path::Path) -> &std::path::Path {
    if data_dir.is_dir() {
        data_dir
    } else {
        data_dir
            .parent()
            .expect("test data directory should have a parent")
    }
}

/// The hook calls the binary directly now, so `apply_plugin_options()` (run
/// before `Config::load()`) must map `CLAUDE_PLUGIN_OPTION_*` into the binary's
/// `RARCANE_*` env vars. Supplying the credentials only via plugin options
/// makes the `missing_rarcane_api_url` / `missing_rarcane_api_key` blocking
/// failures disappear — proving the mapping reaches the loaded config.
#[test]
fn plugin_hook_maps_plugin_options_into_env() {
    let dir = tempdir().unwrap();
    let mut cmd = Command::new(example_bin());
    cmd.env_clear()
        .current_dir(isolated_command_cwd(dir.path()))
        .env("HOME", dir.path())
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("RARCANE_HOME", dir.path())
        .env("RARCANE_MCP_PORT", "0")
        // Supply credentials only via plugin options, not RARCANE_* directly.
        .env(
            "CLAUDE_PLUGIN_OPTION_RARCANE_API_URL",
            "https://api.rarcane.test",
        )
        .env("CLAUDE_PLUGIN_OPTION_RARCANE_API_KEY", "rarcane-secret")
        .env("CLAUDE_PLUGIN_OPTION_API_TOKEN", "mcp-secret");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let blocking: Vec<String> = json["blocking_failures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        !blocking.contains(&"missing_rarcane_api_url".to_string()),
        "API URL option should map into RARCANE_API_URL; blocking: {blocking:?}"
    );
    assert!(
        !blocking.contains(&"missing_rarcane_api_key".to_string()),
        "API key option should map into RARCANE_API_KEY; blocking: {blocking:?}"
    );
}

#[test]
fn setup_plugin_hook_no_repair_emits_json_contract() {
    let dir = tempdir().unwrap();
    let mut cmd = setup_command(dir.path());
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "advisory_failure");
    assert_eq!(json["ran_repair"], false);
    assert_eq!(json["no_repair"], true);
    assert!(json["blocking_failures"].as_array().unwrap().is_empty());
    assert!(
        json["advisory_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["code"] == "env_file_missing")
    );
    assert!(!dir.path().join(".env").exists());
}

#[test]
fn setup_repair_creates_env_file_without_upstream_contact() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("appdata");
    let mut cmd = setup_command(&missing);
    let output = cmd.args(["setup", "repair"]).output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "success");
    assert_eq!(json["ran_repair"], true);
    assert_eq!(json["no_repair"], false);

    let env_file = std::fs::read_to_string(missing.join(".env")).unwrap();
    assert!(env_file.contains("RARCANE_API_URL=https://api.rarcane.test"));
    assert!(env_file.contains("RARCANE_API_KEY=rarcane-secret"));
    assert!(env_file.contains("RARCANE_MCP_TOKEN=mcp-secret"));
    assert_env_file_mode(missing.join(".env").as_path());
}

#[test]
fn setup_repair_replaces_existing_env_file_with_private_mode() {
    let dir = tempdir().unwrap();
    let env_path = dir.path().join(".env");
    fs::write(&env_path, "OLD_VALUE=1\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&env_path, fs::Permissions::from_mode(0o644)).unwrap();

    let mut cmd = setup_command(dir.path());
    let output = cmd.args(["setup", "repair"]).output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let env_file = fs::read_to_string(&env_path).unwrap();
    assert!(!env_file.contains("OLD_VALUE"));
    assert!(env_file.contains("RARCANE_API_URL=https://api.rarcane.test"));
    assert_env_file_mode(&env_path);
}

fn assert_env_file_mode(path: &std::path::Path) {
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

// ── OAuth setup validation (H12) ─────────────────────────────────────────────
//
// These helpers build a Command with OAuth mode enabled and all four OAuth
// credentials present, then selectively omit one field per test to confirm
// the expected blocking-failure code is reported by `setup plugin-hook
// --no-repair`.
//
// Notes:
//   - `setup_command` sets RARCANE_MCP_TOKEN, which normally selects bearer
//     mode.  We override that by adding RARCANE_MCP_AUTH_MODE=oauth.
//   - We omit RARCANE_MCP_TOKEN here so the setup logic enters the OAuth
//     credential-check branch (token takes precedence in bearer mode).
//   - Port is kept at 0 (from setup_command) to avoid mcp_port_in_use noise.

fn oauth_setup_command(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(example_bin());
    cmd.env_clear()
        .current_dir(isolated_command_cwd(data_dir))
        .env("HOME", data_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("RARCANE_HOME", data_dir)
        .env("RARCANE_API_URL", "https://api.rarcane.test")
        .env("RARCANE_API_KEY", "rarcane-secret")
        .env("RARCANE_MCP_PORT", "0")
        .env("RARCANE_MCP_AUTH_MODE", "oauth")
        .env("RARCANE_MCP_PUBLIC_URL", "https://mcp.rarcane.test")
        .env("RARCANE_MCP_GOOGLE_CLIENT_ID", "test-client-id")
        .env("RARCANE_MCP_GOOGLE_CLIENT_SECRET", "test-client-secret")
        .env("RARCANE_MCP_AUTH_ADMIN_EMAIL", "admin@rarcane.test");
    cmd
}

fn blocking_failure_codes(output: &std::process::Output) -> Vec<String> {
    let json: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout not JSON: {e}\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    json["blocking_failures"]
        .as_array()
        .expect("blocking_failures should be an array")
        .iter()
        .map(|f| f["code"].as_str().unwrap_or("").to_string())
        .collect()
}

#[test]
fn oauth_missing_public_url_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    // Remove the public URL so the check fires.
    cmd.env_remove("RARCANE_MCP_PUBLIC_URL");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    // setup exits non-zero when there are blocking failures.
    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_public_url".to_string()),
        "expected missing_oauth_public_url in blocking_failures, got: {codes:?}"
    );
}

#[test]
fn oauth_missing_client_id_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    cmd.env_remove("RARCANE_MCP_GOOGLE_CLIENT_ID");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_client_id".to_string()),
        "expected missing_oauth_client_id in blocking_failures, got: {codes:?}"
    );
}

#[test]
fn oauth_missing_client_secret_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    cmd.env_remove("RARCANE_MCP_GOOGLE_CLIENT_SECRET");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_client_secret".to_string()),
        "expected missing_oauth_client_secret in blocking_failures, got: {codes:?}"
    );
}

#[test]
fn oauth_missing_admin_email_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    cmd.env_remove("RARCANE_MCP_AUTH_ADMIN_EMAIL");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_admin_email".to_string()),
        "expected missing_oauth_admin_email in blocking_failures, got: {codes:?}"
    );
}

// ── write_env OAuth branch (L28) ──────────────────────────────────────────────
//
// When `auth_mode = OAuth` with all OAuth fields set, `setup repair` must
// write a .env that includes RARCANE_MCP_AUTH_MODE=oauth and all four OAuth
// credential lines.

#[test]
fn setup_repair_oauth_writes_oauth_env_lines() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("appdata");
    let mut cmd = oauth_setup_command(&data_dir);
    let output = cmd.args(["setup", "repair"]).output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "success");
    assert_eq!(json["ran_repair"], true);

    let env_file = fs::read_to_string(data_dir.join(".env")).unwrap();
    assert!(
        env_file.contains("RARCANE_MCP_AUTH_MODE=oauth"),
        ".env should contain RARCANE_MCP_AUTH_MODE=oauth"
    );
    assert!(
        env_file.contains("RARCANE_MCP_PUBLIC_URL=https://mcp.rarcane.test"),
        ".env should contain RARCANE_MCP_PUBLIC_URL"
    );
    assert!(
        env_file.contains("RARCANE_MCP_GOOGLE_CLIENT_ID=test-client-id"),
        ".env should contain RARCANE_MCP_GOOGLE_CLIENT_ID"
    );
    assert!(
        env_file.contains("RARCANE_MCP_GOOGLE_CLIENT_SECRET=test-client-secret"),
        ".env should contain RARCANE_MCP_GOOGLE_CLIENT_SECRET"
    );
    assert!(
        env_file.contains("RARCANE_MCP_AUTH_ADMIN_EMAIL=admin@rarcane.test"),
        ".env should contain RARCANE_MCP_AUTH_ADMIN_EMAIL"
    );
    assert_env_file_mode(&data_dir.join(".env"));
}
