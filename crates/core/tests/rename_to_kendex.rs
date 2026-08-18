//! Old-name (vstack) scopes read as an import: they load, their next plan
//! renames them to the new names first, and both generations in one root
//! is a hard error. Global dirs move off `vstack2` on first launch.
#![cfg(unix)]

use std::fs;
use std::path::PathBuf;

use kendex_core::apply;
use kendex_core::engine::{DriftState, audit};
use kendex_core::env::{Env, FakeOs};
use kendex_core::error::CoreError;
use kendex_core::model::Scope;
use kendex_core::rename::migrate_global_dirs;

struct Fixture {
    _tmp: tempfile::TempDir,
    env: Env,
    home: PathBuf,
    project: PathBuf,
}

#[allow(clippy::unwrap_used)]
fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    Fixture {
        env,
        home,
        project,
        _tmp: tmp,
    }
}

const MANIFEST: &str = "schema = 5\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n";

#[test]
#[allow(clippy::unwrap_used)]
fn an_old_name_scope_loads_and_its_plan_renames_first() {
    let f = fixture();
    // A declared skill makes the plan carry real work after the rename
    // prefix — work whose writes must land at the renamed paths.
    let catalog = f.home.join("catalog");
    fs::create_dir_all(catalog.join("skills/gh")).unwrap();
    fs::write(
        catalog.join("skills/gh/SKILL.md"),
        "---\nname: gh\n---\nBody.\n",
    )
    .unwrap();
    let manifest = format!(
        "schema = 5\n\n[sources.cat]\npath = \"{}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[skills.gh]\nsource = \"cat\"\n",
        catalog.display()
    );
    fs::write(f.project.join("vstack.toml"), &manifest).unwrap();
    fs::write(
        f.project.join(".vstack-lock.json"),
        "{\"version\":4,\"entries\":{}}",
    )
    .unwrap();
    fs::write(f.project.join(".gitignore"), "target/\n.vstack-local/\n").unwrap();
    fs::create_dir_all(f.project.join(".vstack-local/skills/handmade")).unwrap();
    fs::write(
        f.project.join(".vstack-local/skills/handmade/SKILL.md"),
        "---\nname: handmade\n---\nBody.\n",
    )
    .unwrap();
    fs::write(f.project.join("untouched.txt"), "bystander").unwrap();

    let scope = Scope::Project {
        root: f.project.clone(),
    };
    let report = audit(&f.env, &scope).unwrap();
    let descriptions: Vec<&str> = report
        .plan
        .ops
        .iter()
        .map(|op| op.description.as_str())
        .collect();
    assert!(
        descriptions[0].starts_with("Rename to kendex")
            && descriptions[0].contains("vstack.toml becomes kendex.toml"),
        "{descriptions:?}"
    );
    let prefix: Vec<&&str> = descriptions
        .iter()
        .take_while(|d| d.starts_with("Rename to kendex"))
        .collect();
    assert_eq!(prefix.len(), 4, "{descriptions:?}");
    assert!(prefix.iter().any(|d| d.contains(".vstack-lock.json")));
    assert!(prefix.iter().any(|d| d.contains(".gitignore")));
    assert!(prefix.iter().any(|d| d.contains(".vstack-local")));

    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(!f.project.join("vstack.toml").exists());
    assert_eq!(
        fs::read_to_string(f.project.join("kendex.toml")).unwrap(),
        manifest
    );
    assert!(!f.project.join(".vstack-lock.json").exists());
    // The install record the same plan wrote followed the rename: the
    // entry sits in the new-name lock, not a recreated old one.
    assert!(
        fs::read_to_string(f.project.join(".kendex-lock.json"))
            .unwrap()
            .contains("skill:gh:claude")
    );
    assert!(f.project.join(".claude/skills/gh").is_symlink());
    assert_eq!(
        fs::read_to_string(f.project.join(".gitignore")).unwrap(),
        "target/\n.kendex-local/\n"
    );
    assert!(!f.project.join(".vstack-local").exists());
    assert!(
        f.project
            .join(".kendex-local/skills/handmade/SKILL.md")
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(f.project.join("untouched.txt")).unwrap(),
        "bystander"
    );

    // Renamed once: the next plan has no generation prefix.
    let after = audit(&f.env, &scope).unwrap();
    assert!(
        after
            .plan
            .ops
            .iter()
            .all(|op| !op.description.starts_with("Rename to kendex")),
        "{:?}",
        after
            .plan
            .ops
            .iter()
            .map(|o| &o.description)
            .collect::<Vec<_>>()
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn the_rename_op_touches_only_the_named_files() {
    let f = fixture();
    // No lock, no gitignore, no local source: the plan renames the
    // manifest and nothing else rides along.
    fs::write(f.project.join("vstack.toml"), MANIFEST).unwrap();
    let scope = Scope::Project {
        root: f.project.clone(),
    };
    let report = audit(&f.env, &scope).unwrap();
    let prefix: Vec<_> = report
        .plan
        .ops
        .iter()
        .take_while(|op| op.description.starts_with("Rename to kendex"))
        .collect();
    assert_eq!(prefix.len(), 1, "{:?}", report.plan.ops);
    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(f.project.join("kendex.toml").is_file());
    assert!(!f.project.join(".kendex-lock.json").exists());
    assert!(!f.project.join(".gitignore").exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn both_generations_in_one_root_is_a_hard_error_naming_both() {
    let f = fixture();
    fs::write(f.project.join("vstack.toml"), MANIFEST).unwrap();
    fs::write(f.project.join("kendex.toml"), MANIFEST).unwrap();
    let scope = Scope::Project {
        root: f.project.clone(),
    };
    let error = audit(&f.env, &scope).unwrap_err();
    assert!(matches!(error, CoreError::BothGenerations { .. }));
    let text = error.to_string();
    assert!(
        text.contains("kendex.toml") && text.contains("vstack.toml"),
        "{text}"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_global_scope_under_the_old_name_gets_the_rename_op() {
    let f = fixture();
    let config = f.home.join(".config/kendex");
    fs::create_dir_all(&config).unwrap();
    fs::write(config.join("vstack.toml"), MANIFEST).unwrap();

    let report = audit(&f.env, &Scope::Global).unwrap();
    assert!(
        report.plan.ops[0]
            .description
            .contains("vstack.toml becomes kendex.toml"),
        "{:?}",
        report.plan.ops
    );
    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(!config.join("vstack.toml").exists());
    assert_eq!(
        fs::read_to_string(config.join("kendex.toml")).unwrap(),
        MANIFEST
    );
}

/// A global skill installed by symlink, with its harness link pointing at
/// the rendered tree under the new app dir. Returns `(link, canonical)`.
#[test]
#[allow(clippy::unwrap_used)]
fn global_dirs_move_off_vstack2_once() {
    let f = fixture();
    fs::create_dir_all(f.home.join(".config/vstack2")).unwrap();
    fs::write(
        f.home.join(".config/vstack2/settings.toml"),
        "projects = []\n",
    )
    .unwrap();
    fs::create_dir_all(f.home.join(".cache/vstack2/sources/mirrors")).unwrap();
    fs::write(f.home.join(".cache/vstack2/sources/mirrors/HEAD"), "ref").unwrap();
    for child in [
        "trash",
        "journal",
        "locks",
        "drift",
        "local-source",
        "rendered",
    ] {
        let dir = f.home.join(".local/share/vstack2").join(child);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("keep"), child).unwrap();
    }

    assert!(migrate_global_dirs(&f.env).unwrap());
    assert_eq!(
        fs::read_to_string(f.home.join(".config/kendex/settings.toml")).unwrap(),
        "projects = []\n"
    );
    assert!(f.home.join(".cache/kendex/sources/mirrors/HEAD").is_file());
    for child in [
        "trash",
        "journal",
        "locks",
        "drift",
        "local-source",
        "rendered",
    ] {
        assert!(
            f.home
                .join(".local/share/kendex")
                .join(child)
                .join("keep")
                .is_file(),
            "{child} did not move"
        );
    }
    assert!(!f.home.join(".config/vstack2").exists());
    assert!(!f.home.join(".cache/vstack2").exists());
    assert!(!f.home.join(".local/share/vstack2").exists());

    // Nothing left to move: the second pass is a no-op.
    assert!(!migrate_global_dirs(&f.env).unwrap());
}

#[test]
#[allow(clippy::unwrap_used)]
fn the_move_never_overwrites_what_the_new_dirs_hold() {
    let f = fixture();
    fs::create_dir_all(f.home.join(".config/vstack2")).unwrap();
    fs::write(f.home.join(".config/vstack2/settings.toml"), "old").unwrap();
    fs::create_dir_all(f.home.join(".config/kendex")).unwrap();
    fs::write(f.home.join(".config/kendex/settings.toml"), "new").unwrap();
    fs::create_dir_all(f.home.join(".local/share/vstack2/journal/scope-a")).unwrap();
    fs::write(
        f.home.join(".local/share/vstack2/journal/scope-a/entry"),
        "old journal",
    )
    .unwrap();
    fs::create_dir_all(f.home.join(".local/share/kendex/journal/scope-a")).unwrap();
    fs::write(
        f.home.join(".local/share/kendex/journal/scope-a/entry"),
        "new journal",
    )
    .unwrap();

    migrate_global_dirs(&f.env).unwrap();
    assert_eq!(
        fs::read_to_string(f.home.join(".config/kendex/settings.toml")).unwrap(),
        "new"
    );
    assert_eq!(
        fs::read_to_string(f.home.join(".config/vstack2/settings.toml")).unwrap(),
        "old"
    );
    assert_eq!(
        fs::read_to_string(f.home.join(".local/share/kendex/journal/scope-a/entry")).unwrap(),
        "new journal"
    );
    assert_eq!(
        fs::read_to_string(f.home.join(".local/share/vstack2/journal/scope-a/entry")).unwrap(),
        "old journal"
    );
}
