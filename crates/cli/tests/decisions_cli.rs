//! Safety decisions from the command line: a finding is printed with the
//! token that names it, the token dismisses it, the registry lists it, and
//! the same record can be taken back — with what the CLI says matching what
//! core records at every step.
#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use kendex_core::engine::ops::{RecordState, list_decisions};
use kendex_core::env::{Env, FakeOs};
use kendex_core::model::Scope;

#[allow(clippy::expect_used)]
fn vstack(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kendex"))
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .output()
        .expect("vstack binary runs")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A project with one installed skill whose finding warns but does not
/// block.
#[allow(clippy::unwrap_used)]
fn project(home: &Path) -> std::path::PathBuf {
    fs::create_dir_all(home.join(".claude/skills")).unwrap();
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    let dir = home.join("catalog/skills/mild");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: mild\ndescription: the mild skill\n---\nRun chmod 777 build.sh first.\n",
    )
    .unwrap();
    let output = vstack(
        home,
        &project,
        &[
            "add",
            home.join("catalog").to_str().unwrap(),
            "--skill",
            "mild",
            "--harness",
            "claude",
            "-y",
        ],
    );
    assert!(output.status.success(), "add failed: {}", stderr(&output));
    project
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_finding_is_dismissed_by_its_token_listed_and_taken_back() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let project = project(home);
    let env = Env::fake(home, FakeOs::Linux);
    let scope = Scope::Project {
        root: project.clone(),
    };

    let listed = vstack(home, &project, &["findings", "--scope", "project"]);
    assert!(listed.status.success(), "{}", stderr(&listed));
    let printed = stderr(&listed);
    assert!(printed.contains("skill mild for Claude Code"), "{printed}");
    let token = printed
        .lines()
        .find_map(|line| line.trim().strip_prefix("token: "))
        .expect("every open finding prints its token")
        .to_owned();
    assert!(token.starts_with("skill:mild:claude#"), "{token}");

    let refused = vstack(home, &project, &["dismiss", &token, "--reason", "because"]);
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("wrong-call"),
        "{}",
        stderr(&refused)
    );

    let dismissed = vstack(
        home,
        &project,
        &["dismiss", &token, "--reason", "wrong-call"],
    );
    assert!(dismissed.status.success(), "{}", stderr(&dismissed));
    let recorded = list_decisions(&env, &scope).unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].state, RecordState::Active);

    let again = stderr(&vstack(home, &project, &["findings", "--scope", "project"]));
    assert!(again.contains("dismissed"), "{again}");
    assert!(
        !again.contains("token: skill:mild:claude#"),
        "a settled finding offers no token: {again}"
    );

    let registry = stderr(&vstack(
        home,
        &project,
        &["decisions", "--scope", "project"],
    ));
    assert!(
        registry.contains("dismissed  skill:mild:claude#"),
        "{registry}"
    );
    assert!(registry.contains("[active]"), "{registry}");
    assert!(registry.contains("wrong-call"), "{registry}");

    // The id the registry printed is what revoke takes.
    let id = registry
        .lines()
        .find_map(|line| line.trim().strip_prefix("dismissed  "))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap()
        .to_owned();
    let revoked = vstack(home, &project, &["decisions", "--revoke", &id]);
    assert!(revoked.status.success(), "{}", stderr(&revoked));
    assert!(list_decisions(&env, &scope).unwrap().is_empty());
    let empty = stderr(&vstack(
        home,
        &project,
        &["decisions", "--scope", "project"],
    ));
    assert!(empty.contains("no decisions recorded"), "{empty}");
}

/// A token from before the content changed dismisses nothing — the same
/// refusal `--allow-unsafe` gives a stale hash.
#[test]
#[allow(clippy::unwrap_used)]
fn a_token_from_before_the_content_changed_dismisses_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let project = project(home);
    let printed = stderr(&vstack(home, &project, &["findings", "--scope", "project"]));
    let token = printed
        .lines()
        .find_map(|line| line.trim().strip_prefix("token: "))
        .unwrap()
        .to_owned();

    let installed = project.join(".claude/skills/mild/SKILL.md");
    let edited = fs::read_to_string(&installed).unwrap() + "\nOne more line.\n";
    fs::write(&installed, edited).unwrap();

    let refused = vstack(home, &project, &["dismiss", &token, "--reason", "intended"]);
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("nothing was changed"),
        "{}",
        stderr(&refused)
    );
    let env = Env::fake(home, FakeOs::Linux);
    assert!(
        list_decisions(&env, &Scope::Project { root: project })
            .unwrap()
            .is_empty()
    );
}
