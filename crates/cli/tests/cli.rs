#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

// Integration-test helpers sit outside #[test] fns, so clippy's
// allow-unwrap-in-tests does not reach them.
#[allow(clippy::expect_used)]
fn vstack(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vstack"))
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .output()
        .expect("vstack binary runs")
}

#[allow(clippy::unwrap_used)]
fn fixture_home() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    fs::create_dir_all(home.join(".claude/agents")).unwrap();
    fs::write(
        home.join(".claude/agents/orch.md"),
        "---\ndescription: boss\n---\n",
    )
    .unwrap();
    fs::create_dir_all(home.join("dev/app/.claude/skills/deploy")).unwrap();
    fs::write(home.join("dev/app/.claude/skills/deploy/SKILL.md"), "# d").unwrap();
    tmp
}

#[test]
fn list_sees_global_and_current_project_scopes() {
    let tmp = fixture_home();
    let home = tmp.path();

    let output = vstack(home, &home.join("dev/app"), &["list"]);
    assert!(output.status.success());
    let table = String::from_utf8_lossy(&output.stderr);
    assert!(table.contains("orch"), "global agent missing: {table}");
    assert!(table.contains("deploy"), "project skill missing: {table}");

    let output = vstack(home, &home.join("dev/app"), &["ls", "--scope", "project"]);
    let table = String::from_utf8_lossy(&output.stderr);
    assert!(!table.contains("orch"));
    assert!(table.contains("deploy"));

    let output = vstack(
        home,
        &home.join("dev/app"),
        &["list", "-g", "--harness", "claude-code"],
    );
    let table = String::from_utf8_lossy(&output.stderr);
    assert!(table.contains("orch"));
    assert!(!table.contains("deploy"));
}

#[test]
fn scope_project_outside_a_project_is_an_error() {
    let tmp = fixture_home();
    let home = tmp.path();
    let output = vstack(home, home, &["list", "--scope", "project"]);
    assert!(!output.status.success());
}

#[test]
fn check_reports_detection_and_exits_zero() {
    let tmp = fixture_home();
    let home = tmp.path();
    let output = vstack(home, &home.join("dev/app"), &["check"]);
    assert!(output.status.success());
    let report = String::from_utf8_lossy(&output.stderr);
    assert!(report.contains("claude:"), "{report}");
    assert!(report.contains("agent"), "{report}");
}

#[test]
fn project_registry_round_trips() {
    let tmp = fixture_home();
    let home = tmp.path();

    let add = vstack(home, home, &["project", "add", "dev/app"]);
    assert!(add.status.success());

    let list = vstack(home, home, &["project", "list"]);
    assert!(String::from_utf8_lossy(&list.stdout).contains("dev/app"));

    let discover = vstack(home, home, &["project", "discover", "dev"]);
    assert!(String::from_utf8_lossy(&discover.stdout).contains("dev/app"));

    let remove = vstack(home, home, &["project", "remove", "dev/app"]);
    assert!(remove.status.success());
    let list = vstack(home, home, &["project", "list"]);
    assert_eq!(String::from_utf8_lossy(&list.stdout).trim(), "");
}

/// A hook can be installed exactly as declared and still do nothing. The one
/// command built for pipelines has to say so rather than tick it green.
#[test]
#[allow(clippy::unwrap_used)]
fn verify_names_an_installation_that_cannot_act() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".github")).unwrap();
    // Walking up from the cwd settles on a directory carrying a harness
    // folder, which is what makes this a project the CLI will act on.
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::create_dir_all(home.join(".copilot")).unwrap();
    fs::write(
        home.join(".copilot/settings.json"),
        "{\"disableAllHooks\": true}",
    )
    .unwrap();

    let catalog = home.join("catalog");
    fs::create_dir_all(catalog.join("hooks")).unwrap();
    fs::write(
        catalog.join("hooks/audit.sh"),
        "#!/usr/bin/env bash\n# ---\n# name: audit\n# event: PreToolUse\n# matcher: Bash\n# description: log shell commands\n# ---\nexit 0\n",
    )
    .unwrap();
    fs::write(
        project.join("vstack.toml"),
        format!(
            "schema = 3\n\n[sources.cat]\npath = \"{}\"\n\n[install]\nharnesses = [\"copilot\"]\nmethod = \"copy\"\n\n[hooks.audit]\nsource = \"cat\"\n",
            catalog.display()
        ),
    )
    .unwrap();

    assert!(vstack(home, &project, &["apply", "-y"]).status.success());
    let output = vstack(home, &project, &["verify"]);
    assert!(output.status.success());
    let printed = String::from_utf8_lossy(&output.stderr);
    assert!(printed.contains("✓ hook audit [copilot]"), "{printed}");
    assert!(
        printed.contains("!") && printed.contains("stays inert"),
        "{printed}"
    );
}
