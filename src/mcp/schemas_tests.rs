use crate::actions::action_names;

use super::tool_definitions;

#[test]
fn schema_action_enum_comes_from_action_metadata() {
    let tools = tool_definitions();
    let enum_values = tools[0]["inputSchema"]["properties"]["action"]["enum"]
        .as_array()
        .expect("action enum should be an array")
        .iter()
        .map(|value| value.as_str().expect("action enum values are strings"))
        .collect::<Vec<_>>();

    assert_eq!(enum_values, action_names());
}

#[test]
fn schema_exposes_arcane_dispatch_fields() {
    let schema = &tool_definitions()[0]["inputSchema"]["properties"];
    assert_eq!(schema["subaction"]["type"], "string");
    assert_eq!(schema["envId"]["type"], "string");
    assert_eq!(schema["id"]["type"], "string");
    assert_eq!(schema["params"]["type"], "object");
}

#[test]
fn schema_disallows_unknown_top_level_properties() {
    let tools = tool_definitions();
    assert_eq!(tools[0]["inputSchema"]["additionalProperties"], false);
}

#[test]
fn schema_exposes_action_specific_subactions_and_requirements() {
    let schema = &tool_definitions()[0]["inputSchema"];
    let rules = schema["allOf"]
        .as_array()
        .expect("conditional action rules");
    let container = rules
        .iter()
        .find(|rule| rule["if"]["properties"]["action"]["const"] == "container")
        .expect("container rule");
    assert!(
        container["then"]["properties"]["subaction"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "list")
    );
    assert!(
        container["then"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "subaction")
    );

    let get = rules
        .iter()
        .find(|rule| {
            rule["if"]["properties"]["action"]["const"] == "container"
                && rule["if"]["properties"]["subaction"]["const"] == "get"
        })
        .expect("container:get rule");
    let required = get["then"]["required"].as_array().unwrap();
    assert!(required.iter().any(|value| value == "envId"));
    assert!(required.iter().any(|value| value == "id"));

    let restore = rules
        .iter()
        .find(|rule| {
            rule["if"]["properties"]["action"]["const"] == "volume"
                && rule["if"]["properties"]["subaction"]["const"] == "restore"
        })
        .expect("volume:restore rule");
    assert!(
        restore["then"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "params")
    );
    assert!(
        restore["then"]["properties"]["params"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "backupId")
    );
}

#[test]
fn environment_single_resource_schema_accepts_id_or_env_id() {
    let rules = tool_definitions()[0]["inputSchema"]["allOf"]
        .as_array()
        .expect("conditional action rules");
    let get = rules
        .iter()
        .find(|rule| {
            rule["if"]["properties"]["action"]["const"] == "environment"
                && rule["if"]["properties"]["subaction"]["const"] == "get"
        })
        .expect("environment:get rule");

    let alternatives = get["then"]["anyOf"]
        .as_array()
        .expect("environment:get should accept either identifier field");
    assert!(
        alternatives
            .iter()
            .any(|alternative| alternative["required"] == serde_json::json!(["id"]))
    );
    assert!(
        alternatives
            .iter()
            .any(|alternative| alternative["required"] == serde_json::json!(["envId"]))
    );
    assert!(
        !get["then"]["required"]
            .as_array()
            .expect("required fields")
            .iter()
            .any(|field| field == "id")
    );
}
