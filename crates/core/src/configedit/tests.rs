use super::*;
#[test]
fn hook_upsert_is_idempotent_and_preserves_unrelated_keys() {
    let start = r#"{"model": "opus", "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "other"}]}]}}"#;
    let edit = ConfigEdit::UpsertHook {
        event: "PreToolUse".into(),
        matcher: Some("Bash".into()),
        command: "bash guard.sh".into(),
        timeout: Some(10),
    };
    let once = edit.apply(start).unwrap();
    assert_eq!(edit.apply(&once).unwrap(), once);
    let value: Value = serde_json::from_str(&once).unwrap();
    assert_eq!(value["model"], "opus");
    assert_eq!(value["hooks"]["Stop"][0]["hooks"][0]["command"], "other");
    assert_eq!(
        value["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "bash guard.sh"
    );
    assert_eq!(value["hooks"]["PreToolUse"][0]["matcher"], "Bash");

    let removed = ConfigEdit::RemoveHook {
        event: None,
        command: "bash guard.sh".into(),
    }
    .apply(&once)
    .unwrap();
    let value: Value = serde_json::from_str(&removed).unwrap();
    assert!(value["hooks"].get("PreToolUse").is_none());
    assert_eq!(value["hooks"]["Stop"][0]["hooks"][0]["command"], "other");
}

#[test]
fn mcp_and_plugin_edits_round_trip() {
    let edit = ConfigEdit::UpsertMcpServer {
        name: "gh".into(),
        value: json!({"command": "gh-mcp", "args": ["--stdio"]}),
    };
    let once = edit
        .apply(r#"{"mcpServers": {"other": {"command": "x"}}}"#)
        .unwrap();
    assert_eq!(edit.apply(&once).unwrap(), once);
    let removed = ConfigEdit::RemoveMcpServer { name: "gh".into() }
        .apply(&once)
        .unwrap();
    let value: Value = serde_json::from_str(&removed).unwrap();
    assert_eq!(value["mcpServers"]["other"]["command"], "x");
    assert!(value["mcpServers"].get("gh").is_none());

    let toggled = ConfigEdit::SetPluginEnabled {
        key: "fmt@main".into(),
        enabled: Some(false),
    }
    .apply("")
    .unwrap();
    let value: Value = serde_json::from_str(&toggled).unwrap();
    assert_eq!(value["enabledPlugins"]["fmt@main"], false);
}

#[test]
fn opencode_instruction_and_codex_feature_edits() {
    let edit = ConfigEdit::OpencodeAddInstruction {
        reference: ".opencode/instructions/vstack-hook-guard.md".into(),
        bash_permission: true,
    };
    let once = edit.apply(r#"{"mcp": {"db": {"type": "local"}}}"#).unwrap();
    assert_eq!(edit.apply(&once).unwrap(), once);
    let value: Value = serde_json::from_str(&once).unwrap();
    assert_eq!(value["mcp"]["db"]["type"], "local");
    assert_eq!(value["permission"]["bash"]["*"], "ask");

    let toml = "# my config\nmodel = \"gpt\"\n\n[features]\nexperimental = true\n";
    let enabled = ConfigEdit::CodexEnableHooksFeature.apply(toml).unwrap();
    assert!(enabled.contains("# my config"));
    assert!(enabled.contains("[features]\nhooks = true\nexperimental = true"));
    assert_eq!(
        ConfigEdit::CodexEnableHooksFeature.apply(&enabled).unwrap(),
        enabled
    );
}

#[test]
fn marker_blocks_upsert_and_strip_cleanly() {
    let base = "# My notes\n";
    let once = upsert_marker_block(base, "pi-hooks", "hook system text");
    assert!(once.starts_with("# My notes\n\n<!-- vstack:append-system pi-hooks begin -->"));
    let twice = upsert_marker_block(&once, "pi-hooks", "hook system text");
    assert_eq!(once, twice);
    assert_eq!(remove_marker_block(&once, "pi-hooks"), base);
}

/// Another tool wrote keys after ours and a handler after ours: a re-apply
/// touches neither position, and removing a key never reorders the rest.
#[test]
fn hook_upsert_refreshes_in_place_and_removal_keeps_key_order() {
    let file = "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Bash\",\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"bash guard.sh\",\n            \"timeout\": 10\n          },\n          {\n            \"type\": \"command\",\n            \"command\": \"theirs\"\n          }\n        ]\n      }\n    ]\n  },\n  \"model\": \"opus\",\n  \"mcpServers\": {\n    \"gh\": {}\n  },\n  \"alwaysThinkingEnabled\": true\n}\n";
    let edit = ConfigEdit::UpsertHook {
        event: "PreToolUse".into(),
        matcher: Some("Bash".into()),
        command: "bash guard.sh".into(),
        timeout: Some(10),
    };
    assert_eq!(edit.apply(file).unwrap(), file);

    let removed = ConfigEdit::RemoveMcpServer { name: "gh".into() }
        .apply(file)
        .unwrap();
    let value: Value = serde_json::from_str(&removed).unwrap();
    let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["hooks", "model", "alwaysThinkingEnabled"]);
}
