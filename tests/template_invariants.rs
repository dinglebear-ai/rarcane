use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|err| panic!("failed to parse {path}: {err}"))
}

#[test]
fn portable_scripts_are_executable_and_documented() {
    let docs = read("scripts/README.md");
    for path in [
        "scripts/check-dependency-updates.sh",
        "scripts/check-file-size.sh",
        "scripts/asciicheck.py",
        "scripts/check-blob-size.py",
        "scripts/check-runtime-current.sh",
        "scripts/validate-plugin-layout.sh",
        "scripts/test-mcp-auth.sh",
        "scripts/pre-release-check.sh",
        "scripts/test-template-features.sh",
        "scripts/check-schema-docs.py",
        "scripts/check-coupled-files.sh",
    ] {
        let metadata = fs::metadata(path).unwrap_or_else(|err| panic!("{path}: {err}"));
        assert!(
            metadata.permissions().mode() & 0o111 != 0,
            "{path} should be executable"
        );
        let basename = Path::new(path).file_name().unwrap().to_string_lossy();
        assert!(
            docs.contains(basename.as_ref()),
            "scripts/README.md should document {basename}"
        );
    }
}

#[test]
fn justfile_exposes_automation_recipes() {
    let justfile = read("Justfile");
    for recipe in [
        "install-hooks:",
        "uninstall-hooks:",
        "deps-check:",
        "blob-size-check:",
        "coupled-files-check:",
        "ascii-check:",
        "ascii-fix:",
        "file-size-check:",
        "schema-docs:",
        "schema-docs-check:",
        "template-features:",
        "template-check:",
        "test-cov:",
        "watch:",
        "runtime-current:",
        "auth-smoke:",
        "pre-release:",
        "up:",
        "down:",
        "release:",
    ] {
        assert!(justfile.contains(recipe), "Justfile missing {recipe}");
    }

    assert!(
        !justfile.contains("install-tools:") && !justfile.contains("bootstrap: install-tools"),
        "development tools are provisioned globally by mise, not installed by project recipes"
    );

    let lefthook = read("lefthook.yml");
    assert!(
        lefthook.contains("mise install"),
        "lefthook should direct missing-tool failures to the mise-managed toolchain"
    );
}

#[test]
fn production_deployment_is_authenticated_and_uses_the_published_image() {
    let compose = read("docker-compose.prod.yml");
    let dockerfile = read("config/Dockerfile");
    let env_example = read(".env.example");

    assert!(
        compose.contains("ghcr.io/dinglebear-ai/rarcane:latest"),
        "production compose must use the canonical organization image"
    );
    assert!(
        compose.contains("RARCANE_MCP_NO_AUTH: \"${RARCANE_MCP_NO_AUTH:-false}\""),
        "production compose must keep authentication enabled by default"
    );
    assert!(
        compose.contains("RARCANE_MCP_TOKEN: \"${RARCANE_MCP_TOKEN:?"),
        "production compose must require a bearer token"
    );
    assert!(!compose.contains("RARCANE_NOAUTH: \"true\""));
    assert!(env_example.contains("RARCANE_MCP_VERSION=v0.4.0"));
    let base_images: Vec<_> = dockerfile
        .lines()
        .filter(|line| line.starts_with("FROM "))
        .collect();
    assert_eq!(
        base_images.len(),
        base_images
            .iter()
            .filter(|line| line.contains("@sha256:"))
            .count(),
        "every Docker base image must be pinned by digest"
    );
}

#[test]
fn plugin_manifests_do_not_have_version_fields() {
    for path in [
        "plugins/rarcane/.claude-plugin/plugin.json",
        "plugins/rarcane/.codex-plugin/plugin.json",
        "plugins/rarcane/gemini-extension.json",
    ] {
        let manifest = json(path);
        assert!(
            !manifest.as_object().unwrap().contains_key("version"),
            "{path} must not contain a version field"
        );
    }
}

#[test]
fn schema_contract_doc_tracks_known_actions() {
    let doc = read("docs/MCP_SCHEMA.md");
    let actions = read("src/actions.rs");
    let schemas = read("src/mcp/schemas.rs");
    for action in [
        "help",
        "status",
        "environment",
        "project",
        "container",
        "image",
        "network",
        "volume",
        "system",
        "image-update",
        "vulnerability",
        "registry",
        "gitops",
    ] {
        assert!(actions.contains(action), "actions.rs missing {action}");
        assert!(
            doc.contains(&format!("`{action}`")),
            "schema doc missing {action}"
        );
    }
    assert!(
        schemas.contains("action_names()"),
        "schemas.rs should derive action enum from action metadata"
    );
}
