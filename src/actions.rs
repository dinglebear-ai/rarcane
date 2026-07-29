use anyhow::Result;
use serde_json::{Value, json};
use std::time::Duration;

use crate::app::ArcaneService;

pub const READ_SCOPE: &str = "rarcane:read";
pub const WRITE_SCOPE: &str = "rarcane:write";
pub const DENY_SCOPE: &str = "rarcane:__deny__";

pub fn scopes_satisfy(token_scopes: &[String], required: &str) -> bool {
    token_scopes
        .iter()
        .any(|s| s == required || (required == READ_SCOPE && s == WRITE_SCOPE))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingAction,
    MissingSubaction { action: String },
    MissingEnvId { action: String, subaction: String },
    MissingId { label: String },
    UnknownAction { action: String },
    UnknownSubaction { action: String, subaction: String },
    McpOnlyAction { action: String },
    WrongType { field: String },
    InvalidPath { field: String },
    DestructiveConfirmationRequired { action: String, subaction: String },
    OutOfRange { field: String, min: u64, max: u64 },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAction => write!(f, "action is required"),
            Self::MissingSubaction { action } => {
                write!(f, "subaction is required for action `{action}`")
            }
            Self::MissingEnvId { action, subaction } => {
                write!(f, "envId is required for {action}:{subaction}")
            }
            Self::MissingId { label } => write!(f, "{label} id is required"),
            Self::UnknownAction { action } => write!(
                f,
                "unknown rarcane action: {action}; use action=help for documentation"
            ),
            Self::UnknownSubaction { action, subaction } => write!(
                f,
                "unknown subaction `{subaction}` for action `{action}`; use action=help subaction={action}"
            ),
            Self::McpOnlyAction { action } => {
                write!(f, "action `{action}` is MCP-only and requires an MCP peer")
            }
            Self::WrongType { field } => write!(f, "`{field}` has the wrong type"),
            Self::InvalidPath { field } => {
                write!(f, "`{field}` must be a relative path without `..` segments")
            }
            Self::DestructiveConfirmationRequired { action, subaction } => write!(
                f,
                "confirmation required for destructive operation {action}:{subaction}; re-run with params.confirm=true or CLI --confirm"
            ),
            Self::OutOfRange { field, min, max } => {
                write!(f, "`{field}` must be between {min} and {max}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTransport {
    Any,
    McpOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyMode {
    None,
    Params,
    ParamsWithoutControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionSpec {
    pub action: &'static str,
    pub subaction: Option<&'static str>,
    pub method: &'static str,
    pub path: &'static str,
    pub required_scope: Option<&'static str>,
    pub transport: ActionTransport,
    pub requires_env: bool,
    pub id_label: Option<&'static str>,
    pub required_params: &'static [&'static str],
    pub destructive: bool,
    pub long_running: bool,
    pub body: BodyMode,
}

impl ActionSpec {
    pub fn key(&self) -> String {
        match self.subaction {
            Some(subaction) => format!("{}:{subaction}", self.action),
            None => self.action.to_string(),
        }
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.long_running.then_some(Duration::from_secs(120))
    }
}

macro_rules! is_long_running {
    ("build") => {
        true
    };
    ("pull") => {
        true
    };
    ("redeploy") => {
        true
    };
    ("prune") => {
        true
    };
    ("scan") => {
        true
    };
    ("create-backup") => {
        true
    };
    ("restore") => {
        true
    };
    ("restore-files") => {
        true
    };
    ("check-all") => {
        true
    };
    ("check-batch") => {
        true
    };
    ("sync") => {
        true
    };
    ($other:tt) => {
        false
    };
}

macro_rules! spec {
    ($action:literal, $sub:tt, $method:literal, $path:literal, $scope:ident, env=$env:literal, id=$id:expr, params=$params:expr, destructive=$destructive:literal, body=$body:ident) => {
        ActionSpec {
            action: $action,
            subaction: Some($sub),
            method: $method,
            path: $path,
            required_scope: Some($scope),
            transport: ActionTransport::Any,
            requires_env: $env,
            id_label: $id,
            required_params: $params,
            destructive: $destructive,
            long_running: is_long_running!($sub),
            body: BodyMode::$body,
        }
    };
    ($action:literal, $sub:tt, $method:literal, $path:literal, $scope:ident, env=$env:literal, id=$id:expr, destructive=$destructive:literal, body=$body:ident) => {
        ActionSpec {
            action: $action,
            subaction: Some($sub),
            method: $method,
            path: $path,
            required_scope: Some($scope),
            transport: ActionTransport::Any,
            requires_env: $env,
            id_label: $id,
            required_params: &[],
            destructive: $destructive,
            long_running: is_long_running!($sub),
            body: BodyMode::$body,
        }
    };
}

pub const ACTION_SPECS: &[ActionSpec] = &[
    ActionSpec {
        action: "help",
        subaction: None,
        method: "GET",
        path: "",
        required_scope: None,
        transport: ActionTransport::Any,
        requires_env: false,
        id_label: None,
        required_params: &[],
        destructive: false,
        long_running: false,
        body: BodyMode::None,
    },
    ActionSpec {
        action: "status",
        subaction: None,
        method: "GET",
        path: "",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
        requires_env: false,
        id_label: None,
        required_params: &[],
        destructive: false,
        long_running: false,
        body: BodyMode::None,
    },
    ActionSpec {
        action: "elicit_name",
        subaction: None,
        method: "GET",
        path: "",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::McpOnly,
        requires_env: false,
        id_label: None,
        required_params: &[],
        destructive: false,
        long_running: false,
        body: BodyMode::None,
    },
    ActionSpec {
        action: "scaffold_intent",
        subaction: None,
        method: "GET",
        path: "",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::McpOnly,
        requires_env: false,
        id_label: None,
        required_params: &[],
        destructive: false,
        long_running: false,
        body: BodyMode::None,
    },
    spec!(
        "environment",
        "list",
        "GET",
        "/environments",
        READ_SCOPE,
        env = false,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "environment",
        "get",
        "GET",
        "/environments/{id}",
        READ_SCOPE,
        env = false,
        id = Some("environment"),
        destructive = false,
        body = None
    ),
    spec!(
        "environment",
        "create",
        "POST",
        "/environments",
        WRITE_SCOPE,
        env = false,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "environment",
        "update",
        "PUT",
        "/environments/{id}",
        WRITE_SCOPE,
        env = false,
        id = Some("environment"),
        destructive = false,
        body = Params
    ),
    spec!(
        "environment",
        "delete",
        "DELETE",
        "/environments/{id}",
        WRITE_SCOPE,
        env = false,
        id = Some("environment"),
        destructive = true,
        body = None
    ),
    spec!(
        "environment",
        "test",
        "POST",
        "/environments/{id}/test",
        READ_SCOPE,
        env = false,
        id = Some("environment"),
        destructive = false,
        body = None
    ),
    spec!(
        "project",
        "list",
        "GET",
        "/environments/{envId}/projects",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "project",
        "get",
        "GET",
        "/environments/{envId}/projects/{id}",
        READ_SCOPE,
        env = true,
        id = Some("project"),
        destructive = false,
        body = None
    ),
    spec!(
        "project",
        "create",
        "POST",
        "/environments/{envId}/projects",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "project",
        "update",
        "PUT",
        "/environments/{envId}/projects/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = false,
        body = Params
    ),
    spec!(
        "project",
        "up",
        "POST",
        "/environments/{envId}/projects/{id}/up",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = false,
        body = None
    ),
    spec!(
        "project",
        "down",
        "POST",
        "/environments/{envId}/projects/{id}/down",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = true,
        body = None
    ),
    spec!(
        "project",
        "restart",
        "POST",
        "/environments/{envId}/projects/{id}/restart",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = true,
        body = None
    ),
    spec!(
        "project",
        "pull",
        "POST",
        "/environments/{envId}/projects/{id}/pull",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = false,
        body = None
    ),
    spec!(
        "project",
        "destroy",
        "DELETE",
        "/environments/{envId}/projects/{id}/destroy",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = true,
        body = None
    ),
    spec!(
        "project",
        "redeploy",
        "POST",
        "/environments/{envId}/projects/{id}/redeploy",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = true,
        body = None
    ),
    spec!(
        "project",
        "build",
        "POST",
        "/environments/{envId}/projects/{id}/build",
        WRITE_SCOPE,
        env = true,
        id = Some("project"),
        destructive = false,
        body = Params
    ),
    spec!(
        "container",
        "list",
        "GET",
        "/environments/{envId}/containers",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "container",
        "get",
        "GET",
        "/environments/{envId}/containers/{id}",
        READ_SCOPE,
        env = true,
        id = Some("container"),
        destructive = false,
        body = None
    ),
    spec!(
        "container",
        "create",
        "POST",
        "/environments/{envId}/containers",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "container",
        "start",
        "POST",
        "/environments/{envId}/containers/{id}/start",
        WRITE_SCOPE,
        env = true,
        id = Some("container"),
        destructive = false,
        body = None
    ),
    spec!(
        "container",
        "stop",
        "POST",
        "/environments/{envId}/containers/{id}/stop",
        WRITE_SCOPE,
        env = true,
        id = Some("container"),
        destructive = true,
        body = None
    ),
    spec!(
        "container",
        "restart",
        "POST",
        "/environments/{envId}/containers/{id}/restart",
        WRITE_SCOPE,
        env = true,
        id = Some("container"),
        destructive = true,
        body = None
    ),
    spec!(
        "container",
        "update",
        "POST",
        "/environments/{envId}/containers/{id}/update",
        WRITE_SCOPE,
        env = true,
        id = Some("container"),
        destructive = false,
        body = None
    ),
    spec!(
        "container",
        "delete",
        "DELETE",
        "/environments/{envId}/containers/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("container"),
        destructive = true,
        body = None
    ),
    spec!(
        "container",
        "stats",
        "GET",
        "/environments/{envId}/containers/counts",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "image",
        "list",
        "GET",
        "/environments/{envId}/images",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "image",
        "get",
        "GET",
        "/environments/{envId}/images/{id}",
        READ_SCOPE,
        env = true,
        id = Some("image"),
        destructive = false,
        body = None
    ),
    spec!(
        "image",
        "pull",
        "POST",
        "/environments/{envId}/images/pull",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "image",
        "delete",
        "DELETE",
        "/environments/{envId}/images/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("image"),
        destructive = true,
        body = None
    ),
    spec!(
        "image",
        "prune",
        "POST",
        "/environments/{envId}/images/prune",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = true,
        body = None
    ),
    spec!(
        "image",
        "scan",
        "POST",
        "/environments/{envId}/images/{id}/vulnerabilities/scan",
        WRITE_SCOPE,
        env = true,
        id = Some("image"),
        destructive = false,
        body = None
    ),
    spec!(
        "network",
        "list",
        "GET",
        "/environments/{envId}/networks",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "network",
        "get",
        "GET",
        "/environments/{envId}/networks/{id}",
        READ_SCOPE,
        env = true,
        id = Some("network"),
        destructive = false,
        body = None
    ),
    spec!(
        "network",
        "create",
        "POST",
        "/environments/{envId}/networks",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "network",
        "delete",
        "DELETE",
        "/environments/{envId}/networks/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("network"),
        destructive = true,
        body = None
    ),
    spec!(
        "network",
        "prune",
        "POST",
        "/environments/{envId}/networks/prune",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = true,
        body = None
    ),
    spec!(
        "volume",
        "list",
        "GET",
        "/environments/{envId}/volumes",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "volume",
        "get",
        "GET",
        "/environments/{envId}/volumes/{id}",
        READ_SCOPE,
        env = true,
        id = Some("volume"),
        destructive = false,
        body = None
    ),
    spec!(
        "volume",
        "create",
        "POST",
        "/environments/{envId}/volumes",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "volume",
        "delete",
        "DELETE",
        "/environments/{envId}/volumes/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("volume"),
        destructive = true,
        body = None
    ),
    spec!(
        "volume",
        "prune",
        "POST",
        "/environments/{envId}/volumes/prune",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = true,
        body = None
    ),
    spec!(
        "volume",
        "browse",
        "GET",
        "/environments/{envId}/volumes/{id}/browse",
        READ_SCOPE,
        env = true,
        id = Some("volume"),
        destructive = false,
        body = None
    ),
    spec!(
        "volume",
        "list-backups",
        "GET",
        "/environments/{envId}/volumes/{id}/backups",
        READ_SCOPE,
        env = true,
        id = Some("volume"),
        destructive = false,
        body = None
    ),
    spec!(
        "volume",
        "create-backup",
        "POST",
        "/environments/{envId}/volumes/{id}/backups",
        WRITE_SCOPE,
        env = true,
        id = Some("volume"),
        destructive = false,
        body = None
    ),
    spec!(
        "volume",
        "delete-backup",
        "DELETE",
        "/environments/{envId}/backups/{backupId}",
        WRITE_SCOPE,
        env = true,
        id = None,
        params = &["backupId"],
        destructive = true,
        body = None
    ),
    spec!(
        "volume",
        "restore",
        "POST",
        "/environments/{envId}/volumes/{id}/backups/{backupId}/restore",
        WRITE_SCOPE,
        env = true,
        id = Some("volume"),
        params = &["backupId"],
        destructive = true,
        body = None
    ),
    spec!(
        "volume",
        "restore-files",
        "POST",
        "/environments/{envId}/volumes/{id}/backups/{backupId}/restore-files",
        WRITE_SCOPE,
        env = true,
        id = Some("volume"),
        params = &["backupId"],
        destructive = true,
        body = ParamsWithoutControl
    ),
    spec!(
        "system",
        "prune",
        "POST",
        "/environments/{envId}/system/prune",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = true,
        body = ParamsWithoutControl
    ),
    spec!(
        "system",
        "start-all",
        "POST",
        "/environments/{envId}/system/containers/start-all",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "system",
        "stop-all",
        "POST",
        "/environments/{envId}/system/containers/stop-all",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = true,
        body = None
    ),
    spec!(
        "system",
        "docker-info",
        "GET",
        "/environments/{envId}/system/docker/info",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "system",
        "convert",
        "POST",
        "/environments/{envId}/system/convert",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "image-update",
        "check-all",
        "POST",
        "/environments/{envId}/image-updates/check-all",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "image-update",
        "check",
        "GET",
        "/environments/{envId}/image-updates/check/{id}",
        READ_SCOPE,
        env = true,
        id = Some("image"),
        destructive = false,
        body = None
    ),
    spec!(
        "image-update",
        "check-batch",
        "POST",
        "/environments/{envId}/image-updates/check-batch",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "image-update",
        "summary",
        "GET",
        "/environments/{envId}/image-updates/summary",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "vulnerability",
        "summary",
        "GET",
        "/environments/{envId}/vulnerabilities/summary",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "vulnerability",
        "list",
        "GET",
        "/environments/{envId}/vulnerabilities/all",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "vulnerability",
        "scanner-status",
        "GET",
        "/environments/{envId}/vulnerabilities/scanner-status",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "vulnerability",
        "ignore",
        "POST",
        "/environments/{envId}/vulnerabilities/ignore",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "vulnerability",
        "unignore",
        "DELETE",
        "/environments/{envId}/vulnerabilities/ignore/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("ignore"),
        destructive = false,
        body = None
    ),
    spec!(
        "vulnerability",
        "list-ignored",
        "GET",
        "/environments/{envId}/vulnerabilities/ignored",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "registry",
        "list",
        "GET",
        "/container-registries",
        READ_SCOPE,
        env = false,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "registry",
        "get",
        "GET",
        "/container-registries/{id}",
        READ_SCOPE,
        env = false,
        id = Some("registry"),
        destructive = false,
        body = None
    ),
    spec!(
        "registry",
        "create",
        "POST",
        "/container-registries",
        WRITE_SCOPE,
        env = false,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "registry",
        "update",
        "PUT",
        "/container-registries/{id}",
        WRITE_SCOPE,
        env = false,
        id = Some("registry"),
        destructive = false,
        body = Params
    ),
    spec!(
        "registry",
        "delete",
        "DELETE",
        "/container-registries/{id}",
        WRITE_SCOPE,
        env = false,
        id = Some("registry"),
        destructive = true,
        body = None
    ),
    spec!(
        "registry",
        "test",
        "POST",
        "/container-registries/{id}/test",
        READ_SCOPE,
        env = false,
        id = Some("registry"),
        destructive = false,
        body = None
    ),
    spec!(
        "gitops",
        "list",
        "GET",
        "/environments/{envId}/gitops/syncs",
        READ_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = None
    ),
    spec!(
        "gitops",
        "get",
        "GET",
        "/environments/{envId}/gitops/syncs/{id}",
        READ_SCOPE,
        env = true,
        id = Some("sync"),
        destructive = false,
        body = None
    ),
    spec!(
        "gitops",
        "create",
        "POST",
        "/environments/{envId}/gitops/syncs",
        WRITE_SCOPE,
        env = true,
        id = None,
        destructive = false,
        body = Params
    ),
    spec!(
        "gitops",
        "update",
        "PUT",
        "/environments/{envId}/gitops/syncs/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("sync"),
        destructive = false,
        body = Params
    ),
    spec!(
        "gitops",
        "delete",
        "DELETE",
        "/environments/{envId}/gitops/syncs/{id}",
        WRITE_SCOPE,
        env = true,
        id = Some("sync"),
        destructive = true,
        body = None
    ),
    spec!(
        "gitops",
        "sync",
        "POST",
        "/environments/{envId}/gitops/syncs/{id}/sync",
        WRITE_SCOPE,
        env = true,
        id = Some("sync"),
        destructive = true,
        body = None
    ),
    spec!(
        "gitops",
        "status",
        "GET",
        "/environments/{envId}/gitops/syncs/{id}/status",
        READ_SCOPE,
        env = true,
        id = Some("sync"),
        destructive = false,
        body = None
    ),
    spec!(
        "gitops",
        "browse",
        "GET",
        "/environments/{envId}/gitops/syncs/{id}/browse",
        READ_SCOPE,
        env = true,
        id = Some("sync"),
        destructive = false,
        body = None
    ),
];

pub fn action_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = ACTION_SPECS.iter().map(|spec| spec.action).collect();
    names.sort_unstable();
    names.dedup();
    names
}

pub fn is_known_action(action: &str) -> bool {
    ACTION_SPECS.iter().any(|spec| spec.action == action)
}

pub fn required_scope_for_action(action: &str) -> Option<&'static str> {
    required_scope_for(action, None)
}

pub fn required_scope_for(action: &str, subaction: Option<&str>) -> Option<&'static str> {
    ACTION_SPECS
        .iter()
        .find(|spec| spec.action == action && spec.subaction == subaction)
        .map(|spec| spec.required_scope)
        .unwrap_or(Some(DENY_SCOPE))
}

pub fn spec_for(action: &str, subaction: Option<&str>) -> Result<&'static ActionSpec> {
    if action == "help" {
        return Ok(&ACTION_SPECS[0]);
    }
    if let Some(spec) = ACTION_SPECS
        .iter()
        .find(|spec| spec.action == action && spec.subaction == subaction)
    {
        return Ok(spec);
    }
    if !is_known_action(action) {
        return Err(ValidationError::UnknownAction {
            action: action.to_owned(),
        }
        .into());
    }
    let subaction = subaction.ok_or_else(|| ValidationError::MissingSubaction {
        action: action.to_owned(),
    })?;
    ACTION_SPECS
        .iter()
        .find(|spec| spec.action == action && spec.subaction == Some(subaction))
        .ok_or_else(|| {
            ValidationError::UnknownSubaction {
                action: action.to_owned(),
                subaction: subaction.to_owned(),
            }
            .into()
        })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArcaneAction {
    pub action: String,
    pub subaction: Option<String>,
    pub env_id: Option<String>,
    pub id: Option<String>,
    pub params: Value,
}

impl ArcaneAction {
    pub fn name(&self) -> &str {
        &self.action
    }

    pub fn from_mcp_args(args: &Value) -> Result<Self> {
        let action = string_field(args, "action")?.ok_or(ValidationError::MissingAction)?;
        Ok(Self {
            action,
            subaction: string_field(args, "subaction")?,
            env_id: string_field(args, "envId")?,
            id: string_field(args, "id")?,
            params: args.get("params").cloned().unwrap_or_else(|| json!({})),
        })
    }

    pub fn from_rest(action: &str, params: &Value) -> Result<Self> {
        if action.is_empty() {
            return Err(ValidationError::MissingAction.into());
        }
        Ok(Self {
            action: action.to_owned(),
            subaction: string_field(params, "subaction")?,
            env_id: string_field(params, "envId")?,
            id: string_field(params, "id")?,
            params: params
                .get("params")
                .cloned()
                .unwrap_or_else(|| params.clone()),
        })
    }
}

pub async fn execute_service_action(
    service: &ArcaneService,
    action: &ArcaneAction,
) -> Result<Value> {
    service.dispatch(action).await
}

pub fn rest_help() -> Value {
    json!({
        "tool": "arcane",
        "actions": action_names(),
        "usage": "rarcane call --action container --subaction list --env-id <env>",
    })
}

pub fn is_validation_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<ValidationError>().is_some()
}

fn string_field(params: &Value, name: &str) -> Result<Option<String>> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
        Some(Value::String(_)) => Ok(None),
        Some(_) => Err(ValidationError::WrongType { field: name.into() }.into()),
    }
}

pub fn validate_relative_path(params: &Value, field: &str) -> Result<()> {
    let Some(path) = params.get(field) else {
        return Ok(());
    };
    let Some(path) = path.as_str() else {
        return Err(ValidationError::WrongType {
            field: field.into(),
        }
        .into());
    };
    if path.starts_with('/') || path.split('/').any(|segment| segment == "..") {
        return Err(ValidationError::InvalidPath {
            field: field.into(),
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
