//! The default catalog moved repositories (vanillagreencom/vstack →
//! vanillagreencom/kendex). A scope still naming the old repository gets
//! one migration write per file — manifest and lock — the first time it is
//! planned, never a per-package conflict; and what the old spelling
//! fetched keeps serving the scope offline.
#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use kendex_core::engine::{DriftState, audit, ops};
use kendex_core::env::{Env, FakeOs};
use kendex_core::lock::{Lock, LockEntry, Reason, SourceRev};
use kendex_core::manifest::{LEGACY_SOURCE_REPO, Method};
use kendex_core::model::{HarnessId, ItemKind, Scope};
use kendex_core::repo_move::MOVE_DESCRIPTION;
use kendex_core::{apply, remote};

#[allow(clippy::unwrap_used)]
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
}

struct Fixture {
    _tmp: tempfile::TempDir,
    env: Env,
    scope: Scope,
    project: PathBuf,
    /// The commit the old-spelling upstream served before it went away.
    commit: String,
}

/// A scope installed entirely before the repository move: the manifest, the
/// lock, and the source cache all carry the old repo spelling — and the
/// upstream is gone, so everything after setup runs offline.
#[allow(clippy::unwrap_used)]
fn pre_move_fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let upstream = home.join("base").join(LEGACY_SOURCE_REPO);
    fs::create_dir_all(upstream.join("skills/gh")).unwrap();
    fs::write(
        upstream.join("skills/gh/SKILL.md"),
        "---\nname: gh\n---\nBody.\n",
    )
    .unwrap();
    git(&upstream, &["init", "--quiet", "-b", "main"]);
    git(&upstream, &["add", "-A"]);
    git(&upstream, &["commit", "--quiet", "-m", "one"]);
    let base = format!("file://{}", home.join("base").display());
    let env = Env::fake(&home, FakeOs::Linux).with_var("KENDEX_GIT_BASE", &base);
    let commit = remote::sync(&env, LEGACY_SOURCE_REPO, None).unwrap().commit;
    fs::remove_dir_all(home.join("base")).unwrap();

    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join("kendex.toml"),
        format!(
            "schema = 5\n\n[sources.vstack]\nrepo = \"{LEGACY_SOURCE_REPO}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[skills.gh]\nsource = \"vstack\"\n\n[forks.skill.zed]\nsource = \"vstack\"\nrepo = \"{LEGACY_SOURCE_REPO}\"\nforked-at = \"2026-01-01T00:00:00Z\"\n"
        ),
    )
    .unwrap();
    let lock = Lock {
        version: kendex_core::lock::LOCK_VERSION,
        entries: [(
            "skill:gh:claude".to_owned(),
            LockEntry {
                name: "gh".to_owned(),
                kind: ItemKind::Skill,
                harness: HarnessId::Claude,
                source: "vstack".to_owned(),
                source_repo: LEGACY_SOURCE_REPO.to_owned(),
                method: Method::Symlink,
                installed_at: "2026-01-01T00:00:00Z".to_owned(),
                source_hash: "pre-move".to_owned(),
                source_commit: Some(commit.clone()),
                rendered_hash: None,
                enabled: true,
                upstream_skills: None,
                emitted: None,
                registration: None,
                reasons: BTreeSet::from([Reason::Requested]),
            },
        )]
        .into(),
        sources: [(
            "vstack".to_owned(),
            SourceRev {
                repo: LEGACY_SOURCE_REPO.to_owned(),
                rev: None,
                commit: commit.clone(),
            },
        )]
        .into(),
        ..Lock::default()
    };
    kendex_core::lock::save(&project.join(".kendex-lock.json"), &lock).unwrap();
    Fixture {
        env,
        scope: Scope::Project {
            root: project.clone(),
        },
        project,
        commit,
        _tmp: tmp,
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_pre_move_scope_gets_one_migration_write_per_file_and_no_conflicts() {
    let f = pre_move_fixture();

    let report = audit(&f.env, &f.scope).unwrap();
    let conflicts: Vec<_> = report
        .drift
        .iter()
        .filter(|row| row.state == DriftState::Conflict)
        .collect();
    assert!(conflicts.is_empty(), "{conflicts:?}");
    let moves = report
        .plan
        .ops
        .iter()
        .filter(|op| op.description == MOVE_DESCRIPTION)
        .count();
    assert_eq!(moves, 1, "{:?}", report.plan.ops);

    apply::execute(&f.env, &report.plan, None).unwrap();

    let manifest = fs::read_to_string(f.project.join("kendex.toml")).unwrap();
    assert!(!manifest.contains(LEGACY_SOURCE_REPO), "{manifest}");
    // The source keeps its declared name; only the repository moves — the
    // fork's provenance included.
    assert!(manifest.contains("[sources.vstack]"), "{manifest}");
    assert!(manifest.contains("vanillagreencom/kendex"), "{manifest}");
    assert!(manifest.contains("[forks.skill.zed]"), "{manifest}");
    let lock = fs::read_to_string(f.project.join(".kendex-lock.json")).unwrap();
    assert!(!lock.contains(LEGACY_SOURCE_REPO), "{lock}");
    assert!(lock.contains("vanillagreencom/kendex"), "{lock}");
    // The install itself converged offline: the old spelling's cache
    // served the moved repository's content.
    assert!(f.project.join(".claude/skills/gh").is_symlink());
    assert!(lock.contains(&f.commit), "{lock}");

    let after = audit(&f.env, &f.scope).unwrap();
    assert!(after.drift.is_empty(), "{:?}", after.drift);
    assert!(
        after.plan.ops.is_empty(),
        "{:?}",
        after
            .plan
            .ops
            .iter()
            .map(|op| &op.description)
            .collect::<Vec<_>>()
    );
}

/// A default add (no source argument) on a scope seeded before the product
/// rename: the source declared under the old default name is still the
/// default, not whichever source happens to sort first.
#[test]
#[allow(clippy::unwrap_used)]
fn a_default_add_falls_back_to_the_legacy_named_source() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();

    let old_default = home.join("old-default");
    fs::create_dir_all(old_default.join("skills/gh")).unwrap();
    fs::write(
        old_default.join("skills/gh/SKILL.md"),
        "---\nname: gh\n---\nBody.\n",
    )
    .unwrap();
    // Sorts before "vstack": without the legacy-name fallback the default
    // add would land here and miss the skill.
    let another = home.join("another");
    fs::create_dir_all(another.join("skills/other")).unwrap();
    fs::write(
        another.join("skills/other/SKILL.md"),
        "---\nname: other\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        project.join("kendex.toml"),
        format!(
            "schema = 5\n\n[sources.another]\npath = \"{}\"\n\n[sources.vstack]\npath = \"{}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n",
            another.display(),
            old_default.display()
        ),
    )
    .unwrap();

    let scope = Scope::Project {
        root: project.clone(),
    };
    let report = ops::add(
        &env,
        &scope,
        &ops::AddRequest {
            skills: vec!["gh".into()],
            ..ops::AddRequest::default()
        },
    )
    .unwrap();
    apply::execute(&env, &report.plan, None).unwrap();
    let manifest = fs::read_to_string(project.join("kendex.toml")).unwrap();
    assert!(manifest.contains("[skills.gh]"), "{manifest}");
    assert!(manifest.contains("source = \"vstack\""), "{manifest}");
}

/// A scope that predates both renames — old file names AND the old repo —
/// gets both migrations in one plan: the generation rename first, then the
/// repository move, whose manifest write lands at the renamed path.
#[test]
#[allow(clippy::unwrap_used)]
fn an_old_name_scope_with_the_old_repo_renames_first_then_moves() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join("vstack.toml"),
        format!(
            "schema = 5\n\n[sources.vstack]\nrepo = \"{LEGACY_SOURCE_REPO}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n"
        ),
    )
    .unwrap();

    let scope = Scope::Project {
        root: project.clone(),
    };
    let report = audit(&env, &scope).unwrap();
    let descriptions: Vec<&str> = report
        .plan
        .ops
        .iter()
        .map(|op| op.description.as_str())
        .collect();
    assert!(
        descriptions[0].starts_with("Rename to kendex"),
        "{descriptions:?}"
    );
    let rename = descriptions
        .iter()
        .position(|d| d.starts_with("Rename to kendex"))
        .unwrap();
    let moved = descriptions
        .iter()
        .position(|d| *d == MOVE_DESCRIPTION)
        .unwrap();
    assert!(rename < moved, "{descriptions:?}");

    apply::execute(&env, &report.plan, None).unwrap();
    assert!(!project.join("vstack.toml").exists());
    let manifest = fs::read_to_string(project.join("kendex.toml")).unwrap();
    assert!(!manifest.contains(LEGACY_SOURCE_REPO), "{manifest}");
    assert!(manifest.contains("vanillagreencom/kendex"), "{manifest}");
}
