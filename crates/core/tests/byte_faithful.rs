//! Invariants 10–12: writes are byte-faithful and idempotent, a refused
//! operation mutates nothing, and an artifact that cannot be compared is
//! reported, never passed.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use kendex_core::apply;
use kendex_core::configedit::ConfigEdit;
use kendex_core::engine::{DriftState, audit};
use kendex_core::env::{Env, FakeOs};
use kendex_core::model::Scope;
use serde_json::json;

/// Applying the same structured edit twice must be byte-identical the
/// second time, trailing newline included — that equality is the drift
/// check for config-entry kinds, and a writer that drops the newline pins
/// corruption forever (the v1 lesson).
#[test]
#[allow(clippy::unwrap_used)]
fn every_config_edit_is_byte_stable_on_reapply() {
    let edits = [
        ConfigEdit::UpsertHook {
            event: "PreToolUse".into(),
            matcher: Some("Bash".into()),
            command: "./guard.sh".into(),
            timeout: Some(10),
        },
        ConfigEdit::UpsertMcpServer {
            name: "gh".into(),
            value: json!({"command": "gh-mcp"}),
        },
        ConfigEdit::SetPluginEnabled {
            key: "fmt@main".into(),
            enabled: Some(true),
        },
        ConfigEdit::OpencodeAddInstruction {
            reference: "instructions/x.md".into(),
            bash_permission: true,
        },
        ConfigEdit::CodexEnableHooksFeature,
        ConfigEdit::UpsertMarkerBlock {
            name: "pi".into(),
            block: "block text".into(),
        },
    ];
    for edit in edits {
        let once = edit.apply("").unwrap();
        let twice = edit.apply(&once).unwrap();
        assert_eq!(once, twice, "{edit:?} must be idempotent");
        assert!(
            once.ends_with('\n'),
            "{edit:?} must keep a trailing newline"
        );
    }
}

struct Fixture {
    _tmp: tempfile::TempDir,
    env: Env,
    scope: Scope,
    project: std::path::PathBuf,
}

#[allow(clippy::unwrap_used)]
fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    let source = home.join("catalog");
    fs::create_dir_all(source.join("agents")).unwrap();
    fs::write(
        source.join("agents/rust.md"),
        "---\nname: rust\ndescription: d\nrole: engineer\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        project.join("kendex.toml"),
        format!(
            "schema = 5\n\n[sources.cat]\npath = \"{}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[agents.rust]\nsource = \"cat\"\n",
            source.display()
        ),
    )
    .unwrap();
    Fixture {
        env,
        scope: Scope::Project {
            root: project.clone(),
        },
        project,
        _tmp: tmp,
    }
}

/// A rejected apply leaves manifest, lock, and install tree byte-identical
/// (invariant 11) — validation precedes mutation, and rollback heals the
/// rest.
#[test]
#[allow(clippy::unwrap_used)]
fn a_refused_apply_leaves_every_surface_byte_identical() {
    let f = fixture();
    let report = audit(&f.env, &f.scope).unwrap();
    let manifest_before = fs::read(f.project.join("kendex.toml")).unwrap();

    // The plan binds to plan-time state; a file appearing at the target
    // after planning must abort the whole apply.
    fs::create_dir_all(f.project.join(".claude/agents")).unwrap();
    fs::write(f.project.join(".claude/agents/rust.md"), "squatter").unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap_err();

    assert_eq!(
        fs::read(f.project.join("kendex.toml")).unwrap(),
        manifest_before
    );
    assert!(!f.project.join(".kendex-lock.json").exists());
    assert_eq!(
        fs::read_to_string(f.project.join(".claude/agents/rust.md")).unwrap(),
        "squatter"
    );
}

/// An installed artifact the engine cannot re-hash is a conflict row —
/// reported uncompared, never counted as passing (invariant 12).
#[test]
#[allow(clippy::unwrap_used)]
fn an_unreadable_artifact_reports_uncompared_not_ok() {
    let f = fixture();
    let report = audit(&f.env, &f.scope).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();
    let installed = f.project.join(".claude/agents/rust.md");
    fs::set_permissions(&installed, fs::Permissions::from_mode(0o000)).unwrap();

    let report = audit(&f.env, &f.scope).unwrap();
    let row = report
        .drift
        .iter()
        .find(|row| row.name == "rust" && row.state == DriftState::Conflict)
        .expect("unreadable artifact is a conflict row");
    assert!(row.detail.contains("cannot be compared"));
    fs::set_permissions(&installed, fs::Permissions::from_mode(0o644)).unwrap();
}
