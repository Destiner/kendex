//! A warning-only install is not held back, and its findings can only be
//! decided once it is on disk. The view carries what the plan would install
//! with findings, so the preview can say what will need a decision once it
//! lands rather than leaving it to be discovered afterwards.
#![cfg(unix)]

use std::fs;

use kendex_app::audit::{apply_scope, view};
use kendex_core::env::{Env, FakeOs};
use kendex_core::model::Scope;

#[test]
#[allow(clippy::unwrap_used)]
fn a_warning_only_install_is_named_before_it_lands() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    let source = home.join("catalog");
    fs::create_dir_all(source.join("skills/mild")).unwrap();
    fs::write(
        source.join("skills/mild/SKILL.md"),
        "---\nname: mild\ndescription: Use this to set things up.\n---\nRun chmod 777 build.sh first.\n",
    )
    .unwrap();
    fs::write(
        project.join("vstack.toml"),
        format!(
            "schema = 5\n\n[sources.cat]\npath = \"{}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"copy\"\n\n[skills.mild]\nsource = \"cat\"\n",
            source.display()
        ),
    )
    .unwrap();
    let scope = Scope::Project {
        root: project.clone(),
    };

    let before = view(&env, &scope);
    assert!(before.held_back.is_empty(), "nothing blocks a warning");
    assert!(
        !before.safety.iter().any(|row| row.name == "mild"),
        "nothing is on disk yet"
    );
    let queued = before
        .queued
        .iter()
        .find(|row| row.name == "mild")
        .expect("the install with a finding is named before it lands");
    assert_eq!(queued.findings.len(), 1);
    assert!(matches!(
        queued.decisions[0].state,
        kendex_core::engine::decisions::DecisionState::Open { .. }
    ));

    apply_scope(&env, &scope, false, Vec::new()).unwrap();
    let after = view(&env, &scope);
    let installed = after.safety.iter().find(|row| row.name == "mild").unwrap();
    assert_eq!(
        installed.review_hash, queued.review_hash,
        "same bytes, same hash"
    );
    assert!(installed.decisions[0].token.is_some());
}
