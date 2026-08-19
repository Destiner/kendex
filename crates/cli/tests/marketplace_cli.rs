#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

#[allow(clippy::expect_used)]
fn kendex(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kendex"))
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        // Keeps the post-subscribe fetch off the network: shorthands
        // resolve under an empty local base and fail fast.
        .env(
            "KENDEX_GIT_BASE",
            format!("file://{}", home.join("base").display()),
        )
        .output()
        .expect("kendex binary runs")
}

#[allow(clippy::unwrap_used)]
fn fixture_home() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    fs::create_dir_all(home.join("dev/app/.claude")).unwrap();
    fs::create_dir_all(home.join("catalog/skills/gh")).unwrap();
    fs::write(
        home.join("catalog/skills/gh/SKILL.md"),
        "---\nname: gh\n---\nBody.\n",
    )
    .unwrap();
    fs::create_dir_all(home.join("catalog/agents")).unwrap();
    fs::write(
        home.join("catalog/agents/helper.md"),
        "---\ndescription: helps\n---\n",
    )
    .unwrap();
    tmp
}

/// The machine-readable listing is versioned and minimal: subscriptions
/// per scope, counts only once a catalog is readable.
#[test]
#[allow(clippy::unwrap_used)]
fn marketplace_list_json_is_versioned_and_stable() {
    let tmp = fixture_home();
    let home = tmp.path();
    let project = home.join("dev/app");
    let catalog = home.join("catalog");
    let catalog_arg = catalog.display().to_string();

    let output = kendex(
        home,
        &project,
        &["marketplace", "subscribe", &catalog_arg, "--name", "cat"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = kendex(
        home,
        &project,
        &["marketplace", "subscribe", "team/tools", "--name", "mkt"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = kendex(
        home,
        &project,
        &["marketplace", "list", "--json", "--scope", "project"],
    );
    assert!(output.status.success());
    let listed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("list --json emits JSON");
    let expected = serde_json::json!({
        "schema": 1,
        "subscriptions": [
            {
                "scope": { "scope": "project", "root": project.display().to_string() },
                "name": "cat",
                "path": catalog_arg,
                "enabled": true,
                "counts": { "agent": 1, "skill": 1 }
            },
            {
                "scope": { "scope": "project", "root": project.display().to_string() },
                "name": "kendex",
                "repo": "vanillagreencom/kendex",
                "enabled": true
            },
            {
                "scope": { "scope": "project", "root": project.display().to_string() },
                "name": "mkt",
                "repo": "team/tools",
                "enabled": true
            }
        ]
    });
    assert_eq!(listed, expected, "{listed:#}");
}

/// Subscribing prints the preview line naming scope, alias, and target,
/// and a full URL declares a remote (the pre-fix heuristic read it as a
/// folder path).
#[test]
#[allow(clippy::unwrap_used)]
fn marketplace_subscribe_names_what_it_declares() {
    let tmp = fixture_home();
    let home = tmp.path();
    let project = home.join("dev/app");

    let output = kendex(
        home,
        &project,
        &[
            "marketplace",
            "subscribe",
            "https://gitlab.example.com/team/catalog.git",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("Subscribes"), "{said}");
    assert!(said.contains("'catalog'"), "{said}");
    let manifest = fs::read_to_string(project.join("kendex.toml")).unwrap();
    assert!(
        manifest.contains("repo = \"https://gitlab.example.com/team/catalog.git\""),
        "{manifest}"
    );
}
