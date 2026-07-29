//! Tool JSON schemas for the MCP rarcane tool.
//!
//! This file defines the action list and input schema for the `arcane` tool.
//! MCP clients inspect this schema to know what arguments are valid.
//!
//! **Template**: rename `rarcane` to your tool name. Add/remove actions and
//! parameters to match your service. Use `"required": [...]` for mandatory args.

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::actions::{ACTION_SPECS, action_names};

/// Cached JSON schema definitions (static data, built once at first call).
static TOOL_DEFINITIONS: OnceLock<Vec<Value>> = OnceLock::new();

/// Return the JSON schema definitions for all tools (cached after first call).
///
/// Returns a `Vec<Value>` where each item is a tool definition object matching
/// the MCP `Tool` schema: `{ name, description, inputSchema }`.
///
/// This is also used by the schema resource (`rarcane://schema/mcp-tool`).
pub(super) fn tool_definitions() -> &'static Vec<Value> {
    TOOL_DEFINITIONS.get_or_init(build_tool_definitions)
}

fn build_tool_definitions() -> Vec<Value> {
    let schema = json!({
        "name": "arcane",
        "description": "Manage Arcane Docker resources. Use action=help for full documentation.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Resource family or help.",
                    "enum": action_names()
                },
                "subaction": {
                    "type": "string",
                    "description": "Operation to perform within the resource family."
                },
                "envId": {
                    "type": "string",
                    "description": "Target Arcane environment ID. Required for environment-scoped domains."
                },
                "id": {
                    "type": "string",
                    "description": "Resource ID for single-resource operations."
                },
                "params": {
                    "type": "object",
                    "description": "Action-specific parameters. Include {\"confirm\": true} for destructive operations.",
                    "additionalProperties": true
                }
            },
            "required": ["action"],
            "additionalProperties": false,
            "allOf": action_rules()
        }
    });
    vec![schema]
}

fn action_rules() -> Vec<Value> {
    let mut rules = action_names()
        .into_iter()
        .filter_map(|action| {
            let specs = ACTION_SPECS
                .iter()
                .filter(|spec| spec.action == action)
                .collect::<Vec<_>>();
            let subactions = specs
                .iter()
                .filter_map(|spec| spec.subaction)
                .collect::<Vec<_>>();
            if subactions.is_empty() {
                return None;
            }
            let required = vec![Value::from("subaction")];
            let then = json!({
                "properties": {
                    "subaction": {"enum": subactions},
                },
                "required": required,
            });
            Some(json!({
                "if": {"properties": {"action": {"const": action}}},
                "then": then,
            }))
        })
        .collect::<Vec<_>>();

    rules.extend(ACTION_SPECS.iter().filter_map(|spec| {
        let subaction = spec.subaction?;
        let mut required = Vec::new();
        if spec.requires_env {
            required.push(Value::from("envId"));
        }
        let environment_id_fallback = spec.action == "environment" && spec.id_label.is_some();
        if spec.id_label.is_some() && !environment_id_fallback {
            required.push(Value::from("id"));
        }
        if !spec.required_params.is_empty() {
            required.push(Value::from("params"));
        }
        let mut then = json!({"required": required});
        if environment_id_fallback {
            then["anyOf"] = json!([
                {"required": ["id"]},
                {"required": ["envId"]}
            ]);
        }
        if !spec.required_params.is_empty() {
            then["properties"] = json!({
                "params": {"required": spec.required_params}
            });
        }
        Some(json!({
            "if": {
                "properties": {
                    "action": {"const": spec.action},
                    "subaction": {"const": subaction}
                },
                "required": ["action", "subaction"]
            },
            "then": then
        }))
    }));
    rules
}

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
