//! The v0.1 → v0.2 schema migration: journaled, transactional, surgical.
#![cfg(unix)]

use std::fs;

use vstack_core::apply;
use vstack_core::engine::audit;
use vstack_core::env::{Env, FakeOs};
use vstack_core::error::CoreError;
use vstack_core::lock::{load as load_lock, lock_path};
use vstack_core::model::Scope;

struct Fixture {
    _tmp: tempfile::TempDir,
    env: Env,
    scope: Scope,
    manifest_path: std::path::PathBuf,
    original: String,
}

#[allow(clippy::unwrap_used)]
fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();

    let source = home.join("catalog");
    fs::create_dir_all(source.join("skills/gh")).unwrap();
    fs::write(
        source.join("skills/gh/SKILL.md"),
        "---\nname: gh\n---\nBody.\n",
    )
    .unwrap();

    // Hand-formatted v0.1 manifest: the upgrade must change the schema line
    // and nothing else.
    let original = format!(
        "# my project setup\nschema = 1\n\n[sources.cat]\npath = \"{}\"   # local catalog\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[skills.gh]\nsource = \"cat\"\n",
        source.display()
    );
    let manifest_path = project.join("vstack.toml");
    fs::write(&manifest_path, &original).unwrap();
    fs::write(
        project.join(".vstack-lock.json"),
        "{\n  \"version\": 1,\n  \"entries\": {}\n}\n",
    )
    .unwrap();

    Fixture {
        env,
        scope: Scope::Project {
            root: project.clone(),
        },
        manifest_path,
        original,
        _tmp: tmp,
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_v01_scope_upgrades_in_place_changing_only_the_schema_line() {
    let f = fixture();
    let report = audit(&f.env, &f.scope).unwrap();
    assert!(
        report
            .plan
            .ops
            .iter()
            .any(|op| op.description.contains("Upgrade vstack.toml"))
    );
    apply::execute(&f.env, &report.plan, None).unwrap();

    let migrated = fs::read_to_string(&f.manifest_path).unwrap();
    assert_eq!(migrated, f.original.replacen("schema = 1", "schema = 2", 1));
    let lock = load_lock(&lock_path(&f.env, &f.scope)).unwrap();
    assert_eq!(lock.version, vstack_core::lock::LOCK_VERSION);
    assert!(lock.entries.contains_key("skill:gh:claude"));

    // Idempotent: the migrated scope plans no further upgrade.
    let again = audit(&f.env, &f.scope).unwrap();
    assert!(
        !again
            .plan
            .ops
            .iter()
            .any(|op| op.description.contains("Upgrade"))
    );
    assert!(again.drift.is_empty());
}

#[test]
#[allow(clippy::unwrap_used)]
fn an_interrupted_migration_rolls_back_byte_identically() {
    let f = fixture();
    let report = audit(&f.env, &f.scope).unwrap();
    let boundaries = report.plan.ops.len();
    for boundary in 0..boundaries {
        let error = apply::execute(&f.env, &report.plan, Some(boundary)).unwrap_err();
        assert!(matches!(error, CoreError::RolledBack { .. }));
        assert_eq!(fs::read_to_string(&f.manifest_path).unwrap(), f.original);
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_newer_schema_refuses_to_load() {
    let f = fixture();
    fs::write(
        &f.manifest_path,
        f.original.replacen("schema = 1", "schema = 99", 1),
    )
    .unwrap();
    assert!(matches!(
        audit(&f.env, &f.scope),
        Err(CoreError::SchemaTooNew { .. })
    ));
}
