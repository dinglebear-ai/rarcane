//! Business service layer.
//!
//! **All business logic lives here.** CLI and MCP are thin shims that call into this.
//!
//! `ArcaneService` owns an `ArcaneClient` and exposes typed methods.
//! If you need caching, retries, data transformation, or validation, do it here —
//! never in `cli.rs` or `mcp/tools.rs`.

use anyhow::Result;
use reqwest::Method;
use serde_json::{Value, json};

use crate::{
    actions::{
        ActionSpec, ActionTransport, ArcaneAction, BodyMode, ValidationError, rest_help, spec_for,
        validate_relative_path,
    },
    arcane::{ArcaneClient, encode_path_segment},
};

// Unit tests live in a sidecar file — see src/app_tests.rs for the pattern.
#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

/// The service layer — wraps the transport client and adds business logic.
///
/// **Template**: rename this to `MyServiceService` (or whatever fits).
/// Add any fields you need: caches, config, metrics, etc.
#[derive(Clone)]
pub struct ArcaneService {
    client: ArcaneClient,
}

#[derive(Debug, Clone)]
pub struct ScaffoldIntent {
    pub display_name: String,
    pub crate_name: String,
    pub binary_name: String,
    pub server_category: String,
    pub env_prefix: String,
    pub auth_kind: String,
    pub host: String,
    pub port: u16,
    pub mcp_transport: String,
    pub mcp_primitives: String,
    pub deployment: String,
    pub plugins: String,
    pub publish_mcp: bool,
    pub crawl_urls: String,
    pub crawl_repos: String,
    pub crawl_search_topics: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElicitedNameOutcome<'a> {
    Accepted(&'a str),
    NoInput,
    Declined,
    Cancelled,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldIntentValidationError {
    message: String,
}

impl ScaffoldIntentValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ScaffoldIntentValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ScaffoldIntentValidationError {}

impl ArcaneService {
    pub fn new(client: ArcaneClient) -> Self {
        Self { client }
    }

    /// Return local server status without leaking Arcane topology or credentials.
    pub async fn status(&self) -> Result<Value> {
        Ok(local_status())
    }

    pub async fn dispatch(&self, action: &ArcaneAction) -> Result<Value> {
        if action.action == "help" {
            return Ok(help_value(action.subaction.as_deref()));
        }
        if action.action == "status" {
            return self.status().await;
        }
        let spec = validate_service_action(&action.action, action.subaction.as_deref())?;
        validate_request(spec, action)?;
        let path = build_path(spec.path, action)?;
        let query = query_params(spec, action)?;
        let body = body_params(spec.body, &action.params);
        let method = Method::from_bytes(spec.method.as_bytes())?;
        let value = self
            .client
            .request(method, &path, query.as_ref(), body.as_ref(), spec.timeout())
            .await?;
        Ok(normalize_response(value))
    }

    /// Build the response for the elicited-name demo after the MCP shim collects input.
    pub fn elicited_name_greeting(&self, outcome: ElicitedNameOutcome<'_>) -> Value {
        match outcome {
            ElicitedNameOutcome::Accepted(name) => {
                let name = name.trim().to_owned();
                if name.is_empty() {
                    json!({
                        "greeting": "Hello, mysterious stranger!",
                        "note": "You submitted an empty name - that's perfectly fine!",
                    })
                } else {
                    json!({
                        "greeting": format!("Hello, {name}! Welcome to the rarcane MCP server."),
                        "name": name,
                    })
                }
            }
            ElicitedNameOutcome::NoInput => json!({
                "greeting": "Hello! (you provided no name - that's okay)",
            }),
            ElicitedNameOutcome::Declined => json!({
                "message": "No problem - you chose not to share your name.",
                "greeting": "Hello, anonymous user!",
            }),
            ElicitedNameOutcome::Cancelled => json!({
                "message": "Elicitation was cancelled.",
                "greeting": "Hello there!",
            }),
            ElicitedNameOutcome::Unsupported => json!({
                "message": "Elicitation is not supported by this MCP client.",
                "hint": "Try a client like Claude.app that supports MCP elicitation (spec 2025-06-18).",
                "fallback_greeting": "Hello, World! (elicitation unavailable)",
            }),
        }
    }

    /// Convert elicited scaffold requirements into the handoff contract consumed by the skill.
    pub fn scaffold_intent(&self, input: ScaffoldIntent) -> Result<Value> {
        validate_scaffold_intent(&input)?;
        let category = normalize_category(&input.server_category);
        let required_surfaces = if category == "application-platform" {
            vec!["api", "cli", "mcp", "web"]
        } else {
            vec!["mcp", "cli"]
        };
        let service_name = input.binary_name.trim().replace('-', "_");
        let env_prefix = input.env_prefix.trim().to_ascii_uppercase();

        Ok(json!({
            "kind": "rarcane_scaffold_intent",
            "schema_version": 1,
            "server_category": category,
            "required_surfaces": required_surfaces,
            "project": {
                "display_name": input.display_name.trim(),
                "crate_name": input.crate_name.trim(),
                "binary_name": input.binary_name.trim(),
                "service_name": service_name,
                "env_prefix": env_prefix,
            },
            "upstream": {
                "base_url_env": format!("{env_prefix}_API_URL"),
                "auth_kind": normalize_auth_kind(&input.auth_kind),
            },
            "runtime": {
                "host": normalize_host(&input.host),
                "port": input.port,
                "mcp_transport": normalize_transport(&input.mcp_transport),
            },
            "mcp_primitives": normalize_primitives(&input.mcp_primitives),
            "deployment": normalize_deployment(&input.deployment),
            "plugins": normalize_plugins(&input.plugins),
            "publish_mcp": input.publish_mcp,
            "crawl_docs": {
                "urls": split_csv(&input.crawl_urls),
                "repos": split_csv(&input.crawl_repos),
                "search_topics": split_csv(&input.crawl_search_topics),
            },
            "handoff": {
                "recommended_skill": "scaffold-project",
                "instructions": "Create an approval-first scaffold plan from this JSON. Do not mutate files until the user approves the plan.",
            },
            "policy": {
                "business_action_minimum_surfaces": ["mcp", "cli"],
                "upstream_client_surfaces": ["mcp", "cli"],
                "application_platform_surfaces": ["api", "cli", "mcp", "web"],
            }
        }))
    }
}

/// Resolve an action that may be dispatched through the generic service/CLI path.
/// MCP-only actions are handled separately by the peer-aware MCP adapter.
pub fn validate_service_action(
    action: &str,
    subaction: Option<&str>,
) -> Result<&'static ActionSpec> {
    let spec = spec_for(action, subaction)?;
    if spec.transport == ActionTransport::McpOnly {
        return Err(ValidationError::McpOnlyAction {
            action: action.to_owned(),
        }
        .into());
    }
    Ok(spec)
}

fn validate_request(spec: &crate::actions::ActionSpec, action: &ArcaneAction) -> Result<()> {
    if spec.requires_env && action.env_id.as_deref().unwrap_or_default().is_empty() {
        return Err(ValidationError::MissingEnvId {
            action: action.action.clone(),
            subaction: action.subaction.clone().unwrap_or_default(),
        }
        .into());
    }
    if let Some(label) = spec.id_label
        && action.id.as_deref().unwrap_or_default().is_empty()
    {
        if spec.action == "environment" {
            // Preserve TypeScript prior art: environment single-resource ops accept envId fallback.
            if action.env_id.as_deref().unwrap_or_default().is_empty() {
                return Err(ValidationError::MissingId {
                    label: label.into(),
                }
                .into());
            }
        } else {
            return Err(ValidationError::MissingId {
                label: label.into(),
            }
            .into());
        }
    }
    for field in spec.required_params {
        let present = action
            .params
            .get(*field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty());
        if !present {
            return Err(ValidationError::MissingId {
                label: (*field).into(),
            }
            .into());
        }
    }
    if matches!(
        (spec.action, spec.subaction),
        ("volume", Some("browse")) | ("gitops", Some("browse"))
    ) {
        validate_relative_path(&action.params, "path")?;
    }
    if spec.action == "image-update" && spec.subaction == Some("check") {
        let has_id = action.id.as_deref().is_some_and(|id| !id.is_empty());
        let has_ref = action
            .params
            .get("imageRef")
            .and_then(Value::as_str)
            .is_some_and(|image_ref| !image_ref.is_empty());
        if !has_id && !has_ref {
            return Err(ValidationError::MissingId {
                label: "image id or params.imageRef".into(),
            }
            .into());
        }
    }
    if spec.destructive && !allow_destructive() && !confirmed(&action.params) {
        return Err(ValidationError::DestructiveConfirmationRequired {
            action: action.action.clone(),
            subaction: action.subaction.clone().unwrap_or_default(),
        }
        .into());
    }
    Ok(())
}

fn build_path(template: &str, action: &ArcaneAction) -> Result<String> {
    let env_id = action.env_id.as_deref().unwrap_or_default();
    let id = action
        .id
        .as_deref()
        .or_else(|| {
            (action.action == "environment")
                .then_some(action.env_id.as_deref())
                .flatten()
        })
        .unwrap_or_default();
    let backup_id = action
        .params
        .get("backupId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut path = template.replace("{envId}", &encode_path_segment(env_id));
    path = path.replace("{id}", &encode_path_segment(id));
    path = path.replace("{backupId}", &encode_path_segment(backup_id));
    if action.action == "image-update"
        && action.subaction.as_deref() == Some("check")
        && action.id.as_deref().unwrap_or_default().is_empty()
    {
        path = format!(
            "/environments/{}/image-updates/check",
            encode_path_segment(env_id)
        );
    }
    Ok(path)
}

fn query_params(spec: &crate::actions::ActionSpec, action: &ArcaneAction) -> Result<Option<Value>> {
    if spec.method != "GET" {
        return Ok(None);
    }
    let mut query = serde_json::Map::new();
    if is_paginated(spec) {
        query.insert(
            "offset".into(),
            Value::from(pagination_value(&action.params, "offset", 0, u64::MAX)?),
        );
        query.insert(
            "limit".into(),
            Value::from(pagination_value(&action.params, "limit", 50, 200)?),
        );
    }
    for key in ["offset", "limit", "sort_order", "query", "path", "imageRef"] {
        if matches!(key, "offset" | "limit") && is_paginated(spec) {
            continue;
        }
        if let Some(value) = action.params.get(key) {
            query.insert(key.to_string(), value.clone());
        }
    }
    Ok((!query.is_empty()).then_some(Value::Object(query)))
}

fn is_paginated(spec: &crate::actions::ActionSpec) -> bool {
    matches!(
        spec.subaction,
        Some("list" | "browse" | "list-backups" | "list-ignored")
    )
}

fn pagination_value(params: &Value, field: &str, default: u64, max: u64) -> Result<u64> {
    let Some(value) = params.get(field) else {
        return Ok(default);
    };
    let value = value.as_u64().ok_or_else(|| ValidationError::WrongType {
        field: field.into(),
    })?;
    if value > max {
        return Err(ValidationError::OutOfRange {
            field: field.into(),
            min: 0,
            max,
        }
        .into());
    }
    Ok(value)
}

fn body_params(mode: BodyMode, params: &Value) -> Option<Value> {
    match mode {
        BodyMode::None => None,
        BodyMode::Params => Some(strip_control_params(params)),
        BodyMode::ParamsWithoutControl => Some(strip_control_params(params)),
    }
}

fn strip_control_params(params: &Value) -> Value {
    let mut object = params.as_object().cloned().unwrap_or_default();
    for key in ["confirm", "offset", "limit", "sort_order", "query"] {
        object.remove(key);
    }
    Value::Object(object)
}

fn confirmed(params: &Value) -> bool {
    params.get("confirm") == Some(&Value::Bool(true))
}

fn allow_destructive() -> bool {
    std::env::var("RARCANE_MCP_ALLOW_DESTRUCTIVE")
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn normalize_response(value: Value) -> Value {
    value.get("data").cloned().unwrap_or(value)
}

fn help_value(domain: Option<&str>) -> Value {
    let mut actions = crate::actions::ACTION_SPECS
        .iter()
        .filter(|spec| domain.is_none_or(|domain| spec.action == domain))
        .map(|spec| {
            let transport = match spec.transport {
                ActionTransport::Any => "any",
                ActionTransport::McpOnly => "mcp-only",
            };
            json!({
                "action": spec.action,
                "subaction": spec.subaction,
                "transport": transport,
                "scope": spec.required_scope,
                "destructive": spec.destructive,
                "requiresEnvId": spec.requires_env,
                "requiresId": spec.id_label,
            })
        })
        .collect::<Vec<_>>();
    actions.sort_by_key(|value| value["action"].as_str().unwrap_or_default().to_owned());
    json!({
        "tool": "arcane",
        "summary": rest_help(),
        "actions": actions,
    })
}

pub fn local_status() -> Value {
    json!({
        "status": "ok",
        "server": "rarcane",
        "upstream": "arcane",
    })
}

pub fn local_help(domain: Option<&str>) -> Value {
    help_value(domain)
}

fn validate_scaffold_intent(input: &ScaffoldIntent) -> Result<()> {
    validate_non_empty("display_name", &input.display_name)?;
    validate_kebab_identifier("crate_name", &input.crate_name)?;
    validate_kebab_identifier("binary_name", &input.binary_name)?;
    validate_env_prefix(&input.env_prefix)?;
    if input.port == 0 {
        return Err(ScaffoldIntentValidationError::new("port must be between 1 and 65535").into());
    }
    validate_urls("crawl_urls", &input.crawl_urls)?;
    validate_urls("crawl_repos", &input.crawl_repos)?;
    Ok(())
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ScaffoldIntentValidationError::new(format!(
            "`{field}` is required and must not be empty"
        ))
        .into());
    }
    Ok(())
}

fn validate_kebab_identifier(field: &str, value: &str) -> Result<()> {
    let value = value.trim();
    validate_non_empty(field, value)?;
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(ScaffoldIntentValidationError::new(format!(
            "`{field}` is required and must not be empty"
        ))
        .into());
    };
    if !first.is_ascii_lowercase()
        || !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(ScaffoldIntentValidationError::new(format!(
            "`{field}` must match ^[a-z][a-z0-9-]*$"
        ))
        .into());
    }
    Ok(())
}

fn validate_env_prefix(value: &str) -> Result<()> {
    let value = value.trim().to_ascii_uppercase();
    validate_non_empty("env_prefix", &value)?;
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(ScaffoldIntentValidationError::new(
            "`env_prefix` is required and must not be empty",
        )
        .into());
    };
    if !first.is_ascii_uppercase()
        || !chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(ScaffoldIntentValidationError::new(
            "`env_prefix` must match ^[A-Z][A-Z0-9_]*$",
        )
        .into());
    }
    Ok(())
}

fn validate_urls(field: &str, value: &str) -> Result<()> {
    for item in split_csv(value) {
        url::Url::parse(&item).map_err(|_| {
            ScaffoldIntentValidationError::new(format!("`{field}` contains invalid URL: {item}"))
        })?;
    }
    Ok(())
}

fn normalize_category(category: &str) -> &'static str {
    let normalized = category.trim().to_ascii_lowercase();
    if normalized.contains("application") || normalized.contains("platform") {
        "application-platform"
    } else {
        "upstream-client"
    }
}

fn normalize_auth_kind(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => "none",
        "api-key" | "apikey" | "api_key" | "api key" | "key" => "api-key",
        "bearer" | "token" => "bearer",
        "oauth" => "oauth",
        "both" => "both",
        _ => "other",
    }
}

fn normalize_host(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "127.0.0.1".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn normalize_transport(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "stdio" => "stdio",
        "http" | "streamable-http" | "streamable_http" => "http",
        _ => "dual",
    }
}

fn normalize_deployment(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "systemd" => "systemd",
        "docker" | "container" | "containers" => "docker",
        _ => "none",
    }
}

fn normalize_primitives(value: &str) -> Vec<String> {
    let requested = split_csv(value);
    let mut primitives = Vec::new();
    for item in requested {
        let primitive = match item.to_ascii_lowercase().as_str() {
            "tools" | "tool" => Some("tools"),
            "resources" | "resource" => Some("resources"),
            "prompts" | "prompt" => Some("prompts"),
            "elicitation" | "elicit" => Some("elicitation"),
            _ => None,
        };
        if let Some(primitive) = primitive {
            let primitive = primitive.to_owned();
            if !primitives.contains(&primitive) {
                primitives.push(primitive);
            }
        }
    }
    if primitives.is_empty() {
        primitives.push("tools".to_owned());
    }
    primitives
}

fn normalize_plugins(value: &str) -> Vec<String> {
    let requested = split_csv(value);
    let mut plugins = Vec::new();
    for item in requested {
        let plugin = match item.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Some("claude"),
            "codex" => Some("codex"),
            "gemini" => Some("gemini"),
            "none" => None,
            _ => None,
        };
        if let Some(plugin) = plugin {
            let plugin = plugin.to_owned();
            if !plugins.contains(&plugin) {
                plugins.push(plugin);
            }
        }
    }
    plugins
}

fn split_csv(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let item = item.to_owned();
        if !items.contains(&item) {
            items.push(item);
        }
    }
    items
}
