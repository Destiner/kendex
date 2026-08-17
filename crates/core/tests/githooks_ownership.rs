//! Ownership edges of the hooks directory: a worktree reached through a
//! symlink is one lease, not two or none; a hand-deleted directory does
//! not strand `core.hooksPath`; the launch pass recovers a crashed
//! hook mutation; the entrypoints refuse v1's shim at commit time.
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
/// provably vstack's, so uninstall takes it back.
#[test]
#[allow(clippy::unwrap_used)]
fn uninstall_takes_back_a_hooks_path_left_pointing_at_a_deleted_directory() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let hooks_dir = w.repo.join(".git/vstack-hooks");
    std::fs::remove_dir_all(&hooks_dir).unwrap();
    assert!(config_value(&w.repo, "core.hooksPath").is_some());

    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("was already gone")),
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
            .any(|l| l.contains("no vstack hooks are installed")),
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
    let key = vstack_core::apply::common_key(&repo.common_dir);
    let journal_dir = vstack_core::apply::journal::journal_dir_for(&w.env.journal_dir(), &key);
    let victim = repo.hooks_dir().join("pre-commit");
    let before = std::fs::read_to_string(&victim).unwrap();
    vstack_core::apply::journal::write(&journal_dir, std::slice::from_ref(&victim)).unwrap();
    std::fs::write(&victim, "torn write").unwrap();

    let recovered = vstack_core::apply::recover_common_journals(&w.env);
    assert_eq!(recovered.len(), 1, "{recovered:?}");
    assert!(recovered[0].0.starts_with("git-common-"));
    assert!(recovered[0].1.as_ref().unwrap());
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), before);

    let again = vstack_core::apply::recover_common_journals(&w.env);
    assert!(again.iter().all(|(_, result)| !result.as_ref().unwrap()));
}

/// The install-time refusal of v1's shim would be hollow if the shim
/// could reappear afterwards and be chained: the entrypoints carry the
/// sentinel and refuse it at commit time.
#[test]
#[allow(clippy::unwrap_used)]
fn the_entrypoints_refuse_v1s_shim_at_commit_time() {
    for entrypoint in [
        githooks::PRE_COMMIT_ENTRYPOINT,
        githooks::COMMIT_MSG_ENTRYPOINT,
    ] {
        let refusal = entrypoint.find(githooks::V1_SENTINEL).unwrap();
        let chain = entrypoint.find("exec \"$next\"").unwrap();
        assert!(refusal < chain, "the shim is refused before it is chained");
    }
}
