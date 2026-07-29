use rarcane::cli::{Command, SetupCommand, parse_args_from};
use serde_json::json;

#[test]
fn call_command_parses() {
    assert_eq!(
        parse_args_from([
            "call",
            "--action",
            "image-update",
            "--subaction",
            "check",
            "--env-id",
            "env-1",
            "--params-json",
            r#"{"imageRef":"nginx:latest"}"#,
        ])
        .unwrap(),
        Some(Command::Call {
            action: "image-update".into(),
            subaction: Some("check".into()),
            env_id: Some("env-1".into()),
            id: None,
            params: json!({"imageRef":"nginx:latest"}),
        })
    );
}

#[test]
fn destructive_confirm_sets_param() {
    let cmd = parse_args_from([
        "call",
        "--action",
        "container",
        "--subaction",
        "stop",
        "--env-id",
        "env-1",
        "--id",
        "ctr",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    match cmd {
        Command::Call { params, .. } => assert_eq!(params["confirm"], true),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn help_domain_parsed() {
    assert_eq!(
        parse_args_from(["help", "--domain", "container"]).unwrap(),
        Some(Command::Help {
            domain: Some("container".into())
        })
    );
}

#[test]
fn existing_operational_commands_parse() {
    assert_eq!(
        parse_args_from(["setup", "plugin-hook", "--no-repair"]).unwrap(),
        Some(Command::Setup(SetupCommand::PluginHook { no_repair: true }))
    );
    assert_eq!(
        parse_args_from(["doctor", "--json"]).unwrap(),
        Some(Command::Doctor { json: true })
    );
}
