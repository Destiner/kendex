//! Edit protection and forking: a hand-edited installation is a conflict,
//! never a casualty — refresh holds it, automatic removals hold it, and
//! the two ways out are explicit: keep it as a fork, or discard the edits.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use vstack_core::apply;
use vstack_core::engine::{DriftCause, DriftState, PlanOptions, audit, fork, plan_scope};
use vstack_core::env::{Env, FakeOs};
use vstack_core::lock::{load as load_lock, lock_path};
use vstack_core::manifest;
use vstack_core::model::{HarnessId, ItemKind, Scope};
use vstack_core::process::Hardened;
use vstack_core::remote;

const REPO: &str = "owner/catalog";

struct World {
    _tmp: tempfile::TempDir,
    env: Env,
    home: PathBuf,
    upstream: PathBuf,
    scope: Scope,
}

#[allow(clippy::unwrap_used)]
fn git(dir: &Path, args: &[&str]) {
    let output = Hardened::git(args, Some(dir)).run().unwrap();
    assert!(output.status.success(), "git {args:?}");
}

#[allow(clippy::unwrap_used)]
fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(
        dir,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "--quiet",
            "-m",
            message,
        ],
    );
}

#[allow(clippy::unwrap_used)]
fn write_skill(dir: &Path, name: &str, body: &str) {
    let skill = dir.join("skills").join(name);
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: about {name}\n---\n{body}\n"),
    )
    .unwrap();
}

#[allow(clippy::unwrap_used)]
fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let upstream = home.join("git").join(REPO);
    fs::create_dir_all(&upstream).unwrap();
    git(&upstream, &["init", "--quiet", "-b", "main"]);
    fs::create_dir_all(home.join(".claude")).unwrap();
    fs::create_dir_all(home.join("app/.claude")).unwrap();
    let base = format!("file://{}", home.join("git").display());
    World {
        env: Env::fake(&home, FakeOs::Linux).with_var("VSTACK_GIT_BASE", &base),
        scope: Scope::Project {
            root: home.join("app"),
        },
        home,
        upstream,
        _tmp: tmp,
    }
}

#[allow(clippy::unwrap_used)]
fn declare(w: &World, body: &str) {
    let path = manifest::manifest_path(&w.env, &w.scope);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!(
            "schema = 3\n\n[sources.cat]\nrepo = \"{REPO}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n{body}"
        ),
    )
    .unwrap();
}

#[allow(clippy::unwrap_used)]
fn sync_and_apply(w: &World) {
    let loaded = manifest::load_for_mutation(&manifest::manifest_path(&w.env, &w.scope))
        .unwrap()
        .unwrap();
    remote::sync_sources(&w.env, &loaded).unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();
}

fn skill_file(w: &World) -> PathBuf {
    w.home.join("app/.agents/skills/gh/SKILL.md")
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_local_edit_survives_refresh_and_reads_as_local_edit() {
    let w = world();
    write_skill(&w.upstream, "gh", "Upstream.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);

    fs::write(skill_file(&w), "my edited version").unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    let row = report.drift.iter().find(|row| row.name == "gh").unwrap();
    assert_eq!(row.state, DriftState::Conflict);
    assert_eq!(row.cause, Some(DriftCause::LocalEdit));
    apply::execute(&w.env, &report.plan, None).unwrap();
    assert_eq!(
        fs::read_to_string(skill_file(&w)).unwrap(),
        "my edited version",
        "an edit is never an automatic casualty"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn edited_and_moved_upstream_reads_as_both_and_discard_is_explicit() {
    let w = world();
    write_skill(&w.upstream, "gh", "One.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);

    fs::write(skill_file(&w), "my edited version").unwrap();
    write_skill(&w.upstream, "gh", "Two.");
    commit(&w.upstream, "two");
    let loaded = manifest::load_for_mutation(&manifest::manifest_path(&w.env, &w.scope))
        .unwrap()
        .unwrap();
    remote::sync_sources(&w.env, &loaded).unwrap();

    let report = audit(&w.env, &w.scope).unwrap();
    let row = report.drift.iter().find(|row| row.name == "gh").unwrap();
    assert_eq!(row.cause, Some(DriftCause::Both), "{row:?}");
    apply::execute(&w.env, &report.plan, None).unwrap();
    assert_eq!(
        fs::read_to_string(skill_file(&w)).unwrap(),
        "my edited version"
    );

    // Discarding is the explicit act.
    let manifest = manifest::load_for_mutation(&manifest::manifest_path(&w.env, &w.scope))
        .unwrap()
        .unwrap();
    let lock = load_lock(&lock_path(&w.env, &w.scope)).unwrap();
    let report = plan_scope(
        &w.env,
        &w.scope,
        &manifest,
        &lock,
        &PlanOptions {
            overwrite_edited: true,
            ..Default::default()
        },
    )
    .unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();
    assert!(fs::read_to_string(skill_file(&w)).unwrap().contains("Two."));
    assert!(audit(&w.env, &w.scope).unwrap().drift.is_empty());
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_hand_made_copy_of_the_desired_bytes_is_clean() {
    let w = world();
    write_skill(&w.upstream, "gh", "One.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);

    // Rewrite the file with its own exact bytes: no edit, nothing to hold.
    let bytes = fs::read(skill_file(&w)).unwrap();
    fs::write(skill_file(&w), &bytes).unwrap();
    assert!(audit(&w.env, &w.scope).unwrap().drift.is_empty());
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_deleted_install_is_missing_not_an_edit() {
    let w = world();
    write_skill(&w.upstream, "gh", "One.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);

    fs::remove_dir_all(w.home.join("app/.agents/skills/gh")).unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    let row = report.drift.iter().find(|row| row.name == "gh").unwrap();
    assert_eq!(row.state, DriftState::Missing, "{row:?}");
    apply::execute(&w.env, &report.plan, None).unwrap();
    assert!(skill_file(&w).is_file(), "a missing install is restored");
}

#[test]
#[allow(clippy::unwrap_used)]
fn fork_keeps_the_name_pauses_updates_and_survives_refresh() {
    let w = world();
    write_skill(&w.upstream, "gh", "Upstream.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);

    fs::write(
        skill_file(&w),
        "---\nname: gh\ndescription: mine\n---\nMy fork.\n",
    )
    .unwrap();
    let plan = fork::fork(&w.env, &w.scope, ItemKind::Skill, "gh", HarnessId::Claude).unwrap();
    apply::execute(&w.env, &plan, None).unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();

    // The fork's bytes live in the local source and render under the name.
    assert!(
        fs::read_to_string(w.home.join("app/.vstack-local/skills/gh/SKILL.md"))
            .unwrap()
            .contains("My fork.")
    );
    assert!(
        fs::read_to_string(skill_file(&w))
            .unwrap()
            .contains("My fork.")
    );
    let text = fs::read_to_string(manifest::manifest_path(&w.env, &w.scope)).unwrap();
    assert!(text.contains("[forks.skill.gh]"), "{text}");
    assert!(text.contains("source = \"local\""));

    // Upstream keeps moving; the fork does not.
    write_skill(&w.upstream, "gh", "Upstream v2.");
    commit(&w.upstream, "two");
    sync_and_apply(&w);
    assert!(
        fs::read_to_string(skill_file(&w))
            .unwrap()
            .contains("My fork.")
    );
    assert!(audit(&w.env, &w.scope).unwrap().drift.is_empty());

    // The updates projection knows it is a fork now, not an update.
    let rows = vstack_core::package::updates::updates(&w.env, &w.scope).unwrap();
    let gh = rows.iter().find(|row| row.name == "gh").unwrap();
    assert!(gh.forked);
    assert!(
        !gh.update_available,
        "a local fork has no remote versions to offer: {gh:?}"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn rename_fork_moves_the_declaration_and_refuses_depended_on_names() {
    let w = world();
    write_skill(&w.upstream, "gh", "Upstream.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);
    fs::write(
        skill_file(&w),
        "---\nname: gh\ndescription: mine\n---\nMine.\n",
    )
    .unwrap();
    let plan = fork::fork(&w.env, &w.scope, ItemKind::Skill, "gh", HarnessId::Claude).unwrap();
    apply::execute(&w.env, &plan, None).unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();

    let plan = fork::rename_fork(&w.env, &w.scope, ItemKind::Skill, "gh", "my-gh").unwrap();
    apply::execute(&w.env, &plan, None).unwrap();
    let text = fs::read_to_string(manifest::manifest_path(&w.env, &w.scope)).unwrap();
    assert!(text.contains("[skills.my-gh]"), "{text}");
    assert!(text.contains("[forks.skill.my-gh]"));
    assert!(!text.contains("[skills.gh]"));
    assert!(
        w.home
            .join("app/.vstack-local/skills/my-gh/SKILL.md")
            .is_file()
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn an_automatic_sweep_never_takes_edited_bytes() {
    let w = world();
    let gh = w.upstream.join("skills/gh/SKILL.md");
    fs::create_dir_all(gh.parent().unwrap()).unwrap();
    fs::write(
        &gh,
        "---\nname: gh\ndescription: about gh\ndependencies:\n  required: [helper]\n---\nParent.\n",
    )
    .unwrap();
    write_skill(&w.upstream, "helper", "Helper.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);
    assert!(
        w.home.join("app/.agents/skills/helper/SKILL.md").is_file(),
        "dependency installed"
    );

    // The user edits the dependency, then upstream drops it: the sweep
    // (a refresh regenerates and sweeps unneeded) must hold the bytes.
    fs::write(
        w.home.join("app/.agents/skills/helper/SKILL.md"),
        "my notes live here now",
    )
    .unwrap();
    fs::write(
        &gh,
        "---\nname: gh\ndescription: about gh\n---\nParent without helper.\n",
    )
    .unwrap();
    commit(&w.upstream, "two");
    let loaded = manifest::load_for_mutation(&manifest::manifest_path(&w.env, &w.scope))
        .unwrap()
        .unwrap();
    remote::sync_sources(&w.env, &loaded).unwrap();
    let report = vstack_core::engine::plan_refresh(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();
    assert_eq!(
        fs::read_to_string(w.home.join("app/.agents/skills/helper/SKILL.md")).unwrap(),
        "my notes live here now",
        "a sweep never takes edited bytes"
    );
}
