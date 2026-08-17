//! The owned hooks directory: provable ownership, receipt-scoped removal,
//! compare-and-swap on `core.hooksPath` in both directions, per-worktree
//! refusals, leases, and crash recovery through the common journal.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use vstack_core::env::{Env, FakeOs};
use vstack_core::githooks;
use vstack_core::process::Hardened;

struct World {
    _tmp: tempfile::TempDir,
    env: Env,
    home: PathBuf,
    repo: PathBuf,
}

#[allow(clippy::unwrap_used)]
fn git(root: &Path, args: &[&str]) {
    let output = Hardened::git(args, Some(root)).run().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[allow(clippy::unwrap_used)]
fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().canonicalize().unwrap();
    let repo = home.join("proj");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    git(&repo, &["config", "user.email", "t@t"]);
    git(&repo, &["config", "user.name", "t"]);
    std::fs::write(repo.join("a.txt"), "hi\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "--quiet", "-m", "feat: base"]);
    World {
        env: Env::fake(&home, FakeOs::Linux),
        home,
        repo,
        _tmp: tmp,
    }
}

#[allow(clippy::unwrap_used)]
fn config_value(repo: &Path, key: &str) -> Option<String> {
    let output = Hardened::git(&["config", "--get", key], Some(repo))
        .run()
        .unwrap();
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
#[allow(clippy::unwrap_used)]
fn fresh_install_writes_entrypoints_receipt_and_hooks_path() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();

    let hooks = w.repo.join(".git/vstack-hooks");
    for name in ["pre-commit", "commit-msg"] {
        let path = hooks.join(name);
        assert!(path.is_file(), "{name} written");
        use std::os::unix::fs::PermissionsExt;
        assert!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o100 != 0,
            "{name} is executable"
        );
    }
    let receipt = githooks::load_receipt(&githooks::Repo::at(&w.repo).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(receipt.hooks_path, hooks.display().to_string());
    assert_eq!(receipt.leases.len(), 1);
    assert_eq!(
        config_value(&w.repo, "core.hooksPath").as_deref(),
        Some(hooks.display().to_string().as_str())
    );

    // Idempotent: a second install changes nothing and adds no lease.
    githooks::install(&w.env, &w.repo).unwrap();
    let again = githooks::load_receipt(&githooks::Repo::at(&w.repo).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(again, receipt);
}

#[test]
#[allow(clippy::unwrap_used)]
fn repair_after_repo_move_points_config_at_the_new_home() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let moved = w.home.join("moved");
    std::fs::rename(&w.repo, &moved).unwrap();

    githooks::install(&w.env, &moved).unwrap();
    let hooks = moved.join(".git/vstack-hooks");
    assert_eq!(
        config_value(&moved, "core.hooksPath").as_deref(),
        Some(hooks.display().to_string().as_str()),
        "repair updates the recorded path — the receipt proves the stale value was ours"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn uninstall_is_receipt_scoped_with_compare_and_swap_both_ways() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let hooks = w.repo.join(".git/vstack-hooks");

    // A user-added file turns uninstall into a refusal, never a
    // half-removal: unsetting hooksPath around it would disable it.
    std::fs::write(hooks.join("pre-push"), "#!/bin/sh\nexit 0\n").unwrap();
    let error = githooks::uninstall(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("did not write"), "{error}");
    assert!(hooks.join("pre-commit").is_file(), "nothing was removed");
    assert!(
        config_value(&w.repo, "core.hooksPath").is_some(),
        "hooksPath untouched on refusal"
    );
    std::fs::remove_file(hooks.join("pre-push")).unwrap();

    // A user-changed hooksPath survives the uninstall (CAS the other way).
    git(&w.repo, &["config", "core.hooksPath", "/custom/hooks"]);
    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(!hooks.exists(), "the owned directory is removed");
    assert_eq!(
        config_value(&w.repo, "core.hooksPath").as_deref(),
        Some("/custom/hooks"),
        "a hand-set hooksPath is never unset"
    );
    assert!(
        report.lines.iter().any(|l| l.contains("left alone")),
        "{report:?}"
    );

    // The plain path: install then uninstall unsets what we set.
    git(&w.repo, &["config", "--unset", "core.hooksPath"]);
    githooks::install(&w.env, &w.repo).unwrap();
    githooks::uninstall(&w.env, &w.repo).unwrap();
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);
    assert!(!w.repo.join(".git/vstack-hooks").exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn pre_existing_or_symlinked_directory_is_a_refusal_not_an_adoption() {
    let w = world();
    let hooks = w.repo.join(".git/vstack-hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    std::fs::write(hooks.join("pre-commit"), "#!/bin/sh\nexit 0\n").unwrap();
    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("refusal"), "{error}");

    std::fs::remove_dir_all(&hooks).unwrap();
    std::os::unix::fs::symlink(w.home.join("elsewhere"), &hooks).unwrap();
    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("symlink"), "{error}");
}

#[test]
#[allow(clippy::unwrap_used)]
fn v1_shim_is_a_decommission_not_a_chain() {
    let w = world();
    let v1 = w.repo.join(".git/hooks/pre-commit");
    std::fs::create_dir_all(v1.parent().unwrap()).unwrap();
    std::fs::write(&v1, "#!/bin/sh\n# vstack-guards-hook\nexec guards\n").unwrap();
    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("v1"), "{error}");
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_worktree_resolving_a_foreign_hooks_path_is_a_refusal() {
    let w = world();
    git(&w.repo, &["worktree", "add", "--quiet", "../linked"]);
    let linked = w.home.join("linked");
    // Per-worktree config: only this worktree sees the foreign value.
    git(&linked, &["config", "extensions.worktreeConfig", "true"]);
    git(
        &linked,
        &["config", "--worktree", "core.hooksPath", "/their/hooks"],
    );
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);

    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(
        error.to_string().contains("/their/hooks"),
        "the effective value in every worktree is checked: {error}"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn leases_keep_the_install_armed_until_the_last_release() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    git(&w.repo, &["worktree", "add", "--quiet", "../linked"]);
    let linked = w.home.join("linked");
    githooks::install(&w.env, &linked).unwrap();

    let repo = githooks::Repo::at(&w.repo).unwrap();
    assert_eq!(
        githooks::load_receipt(&repo).unwrap().unwrap().leases.len(),
        2
    );

    // One consumer leaves: still armed.
    let report = githooks::uninstall(&w.env, &linked).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("stay armed")),
        "{report:?}"
    );
    assert!(config_value(&w.repo, "core.hooksPath").is_some());

    // The last lease goes: disarmed.
    githooks::uninstall(&w.env, &w.repo).unwrap();
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);
    assert!(!w.repo.join(".git/vstack-hooks").exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_dead_worktrees_lease_is_reaped_in_passing() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    git(&w.repo, &["worktree", "add", "--quiet", "../linked"]);
    let linked = w.home.join("linked");
    githooks::install(&w.env, &linked).unwrap();

    // The linked worktree vanishes without ever releasing its lease.
    std::fs::remove_dir_all(&linked).unwrap();
    git(&w.repo, &["worktree", "prune"]);

    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("reaped")),
        "{report:?}"
    );
    assert_eq!(
        config_value(&w.repo, "core.hooksPath"),
        None,
        "a dead lease must not keep the install armed forever"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_crashed_install_is_recovered_through_the_common_journal() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let repo = githooks::Repo::at(&w.repo).unwrap();
    let hooks = repo.hooks_dir();

    // Simulate a crash mid-mutation: the common journal holds the
    // pre-image, the mutation half-landed, the journal was never cleared.
    let key = vstack_core::apply::common_key(&repo.common_dir);
    let journal_dir = vstack_core::apply::journal::journal_dir_for(&w.env.journal_dir(), &key);
    let victim = hooks.join("pre-commit");
    let before = std::fs::read_to_string(&victim).unwrap();
    vstack_core::apply::journal::write(&journal_dir, std::slice::from_ref(&victim)).unwrap();
    std::fs::write(&victim, "torn write").unwrap();

    // Another worktree's next hook mutation recovers first, under the same
    // common lock the crash held.
    git(&w.repo, &["worktree", "add", "--quiet", "../linked"]);
    githooks::install(&w.env, &w.home.join("linked")).unwrap();
    assert_eq!(
        std::fs::read_to_string(&victim).unwrap(),
        before,
        "the torn write was rolled back before the new mutation"
    );
}
