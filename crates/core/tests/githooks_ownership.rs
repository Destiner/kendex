//! Ownership edges of the hooks directory: a worktree reached through a
//! symlink is one lease, not two or none; a hand-deleted directory does
//! not strand `core.hooksPath`; the launch pass recovers a crashed
//! hook mutation; the entrypoints refuse v1's shim at commit time.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use kendex_core::env::{Env, FakeOs};
use kendex_core::githooks;
use kendex_core::process::Hardened;

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

/// Leases and git's registry compare paths as text; the install seen
/// through a symlink must record the same spelling the registry lists,
/// or the live worktree reads as dead and the last lease disarms early.
#[test]
#[allow(clippy::unwrap_used)]
fn a_worktree_reached_through_a_symlink_is_one_live_lease() {
    let w = world();
    let link = w.home.join("link");
    std::os::unix::fs::symlink(&w.repo, &link).unwrap();
    githooks::install(&w.env, &link).unwrap();
    git(&w.repo, &["worktree", "add", "--quiet", "../linked"]);
    let linked = w.home.join("linked");
    githooks::install(&w.env, &linked).unwrap();

    let receipt = githooks::load_receipt(&githooks::Repo::at(&w.repo).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(receipt.leases.len(), 2, "{:?}", receipt.leases);
    assert!(
        receipt.leases.contains(&w.repo.display().to_string()),
        "the lease is recorded under the canonical path: {:?}",
        receipt.leases
    );

    let report = githooks::uninstall(&w.env, &linked).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("stay armed")),
        "the symlink-installed worktree is alive, not reaped: {report:?}"
    );
    assert!(!report.lines.iter().any(|l| l.contains("reaped")));
    assert!(config_value(&w.repo, "core.hooksPath").is_some());
}

/// The entrypoint's own fail-closed message tells a user how to remove
/// the checks by hand; doing only half leaves git resolving hooks from a
/// directory that is gone — every hook silently off. The config value is
/// provably kendex's, so uninstall takes it back.
#[test]
#[allow(clippy::unwrap_used)]
fn uninstall_takes_back_a_hooks_path_left_pointing_at_a_deleted_directory() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let hooks_dir = w.repo.join(".git/kendex-hooks");
    std::fs::remove_dir_all(&hooks_dir).unwrap();
    assert!(config_value(&w.repo, "core.hooksPath").is_some());

    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("no receipt behind it; unset")),
        "{report:?}"
    );
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);

    // A hooksPath that is not ours is never touched, receipt or not.
    git(&w.repo, &["config", "core.hooksPath", "/their/hooks"]);
    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("no kendex hooks are installed")),
        "{report:?}"
    );
    assert_eq!(
        config_value(&w.repo, "core.hooksPath").as_deref(),
        Some("/their/hooks")
    );
}

/// A crash mid-mutation journals under the repository's common dir, not
/// a scope; the launch pass must find it there, or core.hooksPath stays
/// live over a torn entrypoint until someone reruns install by hand.
#[test]
#[allow(clippy::unwrap_used)]
fn the_launch_pass_recovers_a_crashed_hook_mutation() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let repo = githooks::Repo::at(&w.repo).unwrap();
    let key = kendex_core::apply::common_key(&repo.common_dir);
    let journal_dir = kendex_core::apply::journal::journal_dir_for(&w.env.journal_dir(), &key);
    let victim = repo.hooks_dir().unwrap().join("pre-commit");
    let before = std::fs::read_to_string(&victim).unwrap();
    kendex_core::apply::journal::write(&journal_dir, std::slice::from_ref(&victim)).unwrap();
    std::fs::write(&victim, "torn write").unwrap();

    let recovered = kendex_core::apply::recover_common_journals(&w.env).unwrap();
    assert_eq!(recovered.len(), 1, "{recovered:?}");
    assert!(
        recovered[0].0.starts_with("git-common-proj-"),
        "the key names the repository a person recognizes: {}",
        recovered[0].0
    );
    assert!(recovered[0].1.as_ref().unwrap());
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), before);

    let again = kendex_core::apply::recover_common_journals(&w.env).unwrap();
    assert!(again.iter().all(|(_, result)| !result.as_ref().unwrap()));

    // A journal dir that cannot be listed is an error, not an empty list.
    std::fs::remove_dir_all(w.env.journal_dir()).unwrap();
    std::fs::write(w.env.journal_dir(), "not a directory").unwrap();
    assert!(kendex_core::apply::recover_common_journals(&w.env).is_err());
}

/// git answers `--git-common-dir` relative to where it was asked, not to
/// the top level: from a subdirectory the answer is `../.git`, and joined
/// onto the top level it names the parent's repository — which, when the
/// parent is a repository too, would be armed instead of this one.
#[test]
#[allow(clippy::unwrap_used)]
fn install_from_a_subdirectory_arms_this_repository_not_its_parent() {
    let w = world();
    let inner = w.repo.join("inner");
    std::fs::create_dir_all(inner.join("sub")).unwrap();
    git(&inner, &["init", "--quiet", "-b", "main"]);
    githooks::install(&w.env, &inner.join("sub")).unwrap();
    assert!(inner.join(".git/kendex-hooks/receipt.json").is_file());
    assert!(!w.repo.join(".git/kendex-hooks").exists());
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);
    assert!(
        config_value(&inner, "core.hooksPath")
            .unwrap()
            .ends_with("inner/.git/kendex-hooks")
    );
}

/// A worktree whose directory is gone stays in git's registry as prunable
/// until someone prunes. It is dead here: its lease is reaped and its
/// config is not asked for — install and uninstall both keep working
/// without a manual `git worktree prune` first.
#[test]
#[allow(clippy::unwrap_used)]
fn a_prunable_worktree_is_dead_for_leases_and_refusals() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    git(&w.repo, &["worktree", "add", "--quiet", "../linked"]);
    let linked = w.home.join("linked");
    githooks::install(&w.env, &linked).unwrap();
    std::fs::remove_dir_all(&linked).unwrap();

    githooks::install(&w.env, &w.repo).unwrap();
    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("reaped")),
        "{report:?}"
    );
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);
    assert!(!w.repo.join(".git/kendex-hooks").exists());
}

/// The install-time refusal of v1's shim would be hollow if the shim
/// could reappear afterwards and be chained: the entrypoints carry the
/// sentinel and refuse it at commit time.
#[test]
#[allow(clippy::unwrap_used)]
fn the_entrypoints_refuse_v1s_shim_at_commit_time() {
    for hook in githooks::HOOKS {
        let entrypoint = githooks::entrypoint(hook);
        let refusal = entrypoint.find(githooks::V1_SENTINEL).unwrap();
        let chain = entrypoint.find("exec \"$next\"").unwrap();
        assert!(refusal < chain, "the shim is refused before it is chained");
        assert!(entrypoint.contains(&format!("kendex guard run {hook}")));
    }
    // Install refuses the same set of shims the entrypoints refuse, or a
    // repository installs cleanly and then fails closed on every commit.
    let w = world();
    let v1 = w.repo.join(".git/hooks/commit-msg");
    std::fs::create_dir_all(v1.parent().unwrap()).unwrap();
    std::fs::write(&v1, "#!/bin/sh\n# vstack-guards-hook\nexec guards\n").unwrap();
    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("v1"), "{error}");
}

/// Ownership without a receipt is proven by content: a directory holding
/// nothing but kendex's own entrypoints byte for byte is repaired by
/// install and taken back by uninstall; one holding anything else is
/// refused by install and left in place — named — by uninstall, which
/// still takes back the config value that is ours by name.
#[test]
#[allow(clippy::unwrap_used)]
fn a_receiptless_directory_is_judged_by_its_contents() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let hooks_dir = w.repo.join(".git/kendex-hooks");
    std::fs::remove_file(hooks_dir.join("receipt.json")).unwrap();
    githooks::install(&w.env, &w.repo).unwrap();
    assert!(hooks_dir.join("receipt.json").is_file(), "repaired");

    std::fs::remove_file(hooks_dir.join("receipt.json")).unwrap();
    std::fs::remove_file(hooks_dir.join("commit-msg")).unwrap();
    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("only kendex's own entrypoints")),
        "{report:?}"
    );
    assert!(!hooks_dir.exists());
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);

    // A foreign file: install refuses; uninstall unsets the config value it
    // can prove is ours and leaves the directory, naming the file.
    githooks::install(&w.env, &w.repo).unwrap();
    std::fs::remove_file(hooks_dir.join("receipt.json")).unwrap();
    std::fs::write(hooks_dir.join("theirs"), "#!/bin/sh\n").unwrap();
    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("theirs"), "{error}");
    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("theirs")),
        "{report:?}"
    );
    assert!(hooks_dir.join("theirs").is_file());
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);
}

/// A hook slot that has become a symlink is not a file kendex wrote: a
/// repair must never write through it to wherever it points.
#[test]
#[allow(clippy::unwrap_used)]
fn a_symlinked_hook_slot_is_a_refusal_not_a_write_through() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let slot = w.repo.join(".git/kendex-hooks/pre-commit");
    let target = w.home.join("target.txt");
    std::fs::write(&target, "mine\n").unwrap();
    std::fs::remove_file(&slot).unwrap();
    std::os::unix::fs::symlink(&target, &slot).unwrap();
    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("symlink"), "{error}");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "mine\n");
}

/// A hooks directory that has become a symlink is not a directory kendex
/// created: repair must refuse it as install does, never write through
/// it to wherever it points.
#[test]
#[allow(clippy::unwrap_used)]
fn a_symlinked_hooks_directory_is_a_refusal_for_repair() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let hooks_dir = w.repo.join(".git/kendex-hooks");
    let elsewhere = w.home.join("elsewhere");
    std::fs::rename(&hooks_dir, &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &hooks_dir).unwrap();
    // Old-generation bytes behind the link: still ours by content, so a
    // rewrite through the link would visibly change them.
    for hook in githooks::HOOKS {
        std::fs::write(elsewhere.join(hook), githooks::old_entrypoint(hook)).unwrap();
    }

    let error = githooks::repair(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("symlink"), "{error}");
    for hook in githooks::HOOKS {
        assert_eq!(
            std::fs::read_to_string(elsewhere.join(hook)).unwrap(),
            githooks::old_entrypoint(hook),
            "nothing was written through the link"
        );
    }
}

/// Uninstall from a worktree that never held a lease says so and rewrites
/// nothing; a repository with nothing of ours answers without any lock.
#[test]
#[allow(clippy::unwrap_used)]
fn uninstall_from_a_worktree_without_a_lease_changes_nothing() {
    let w = world();
    let before = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert_eq!(
        before.lines,
        ["no kendex hooks are installed in this repository"]
    );

    githooks::install(&w.env, &w.repo).unwrap();
    git(&w.repo, &["worktree", "add", "--quiet", "../linked"]);
    let receipt_path = w.repo.join(".git/kendex-hooks/receipt.json");
    let receipt = std::fs::read_to_string(&receipt_path).unwrap();
    let report = githooks::uninstall(&w.env, &w.home.join("linked")).unwrap();
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("never enabled the commit checks")),
        "{report:?}"
    );
    assert_eq!(std::fs::read_to_string(&receipt_path).unwrap(), receipt);
    assert!(config_value(&w.repo, "core.hooksPath").is_some());
}
