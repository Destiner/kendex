//! Repos armed by the vstack-named binary: their `vstack-hooks` directory
//! and receipt stay where they are, repair rewrites the entrypoints to
//! call the binary that exists going forward, and ownership by content
//! recognizes the old bytes — repaired by install, taken back by
//! uninstall. Foreign files refuse exactly as they do in the new
//! generation.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use kendex_core::env::{Env, FakeOs};
use kendex_core::githooks;
use kendex_core::process::Hardened;

struct World {
    _tmp: tempfile::TempDir,
    env: Env,
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

/// An install exactly as the vstack-named binary left it: `vstack-hooks`
/// holding the old entrypoint bytes, with or without its receipt, with
/// `core.hooksPath` pointing at it.
#[allow(clippy::unwrap_used)]
fn old_generation_install(repo: &Path, with_receipt: bool) -> PathBuf {
    let hooks_dir = repo.join(".git/vstack-hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    use std::os::unix::fs::PermissionsExt;
    for hook in githooks::HOOKS {
        let path = hooks_dir.join(hook);
        std::fs::write(&path, githooks::old_entrypoint(hook)).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    if with_receipt {
        let receipt = githooks::Receipt {
            schema: 1,
            hooks_path: hooks_dir.display().to_string(),
            files: githooks::HOOKS
                .iter()
                .map(|name| (*name).to_owned())
                .chain(std::iter::once("receipt.json".to_owned()))
                .collect(),
            leases: [repo.display().to_string()].into(),
        };
        let mut text = serde_json::to_string_pretty(&receipt).unwrap();
        text.push('\n');
        std::fs::write(hooks_dir.join("receipt.json"), text).unwrap();
    }
    git(
        repo,
        &["config", "core.hooksPath", &hooks_dir.display().to_string()],
    );
    hooks_dir
}

/// Repair rewrites the receipt-listed entrypoints to the new bytes and
/// nothing else: the directory keeps its old name, `core.hooksPath` and
/// the receipt keep pointing at it, and the lease survives.
#[test]
#[allow(clippy::unwrap_used)]
fn repair_rewrites_an_old_generation_install_in_place() {
    let w = world();
    let hooks_dir = old_generation_install(&w.repo, true);
    let receipt_before = std::fs::read_to_string(hooks_dir.join("receipt.json")).unwrap();

    let report = githooks::repair(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("rewritten")),
        "{report:?}"
    );
    for hook in githooks::HOOKS {
        let path = hooks_dir.join(hook);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            githooks::entrypoint(hook),
            "{hook} now calls the binary that exists going forward"
        );
        use std::os::unix::fs::PermissionsExt;
        assert!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o100 != 0,
            "{hook} stays executable"
        );
    }
    assert_eq!(
        config_value(&w.repo, "core.hooksPath").as_deref(),
        Some(hooks_dir.display().to_string().as_str()),
        "repair rewrites contents, it never moves what the config points at"
    );
    assert_eq!(
        std::fs::read_to_string(hooks_dir.join("receipt.json")).unwrap(),
        receipt_before,
        "the receipt records paths that are still true"
    );
}

/// Without a receipt there is nothing to rewrite under: repair refuses
/// rather than adopting or creating — install owns those paths.
#[test]
#[allow(clippy::unwrap_used)]
fn repair_without_a_receipt_is_a_refusal() {
    let w = world();
    let error = githooks::repair(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("nothing to repair"), "{error}");

    old_generation_install(&w.repo, false);
    let error = githooks::repair(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("nothing to repair"), "{error}");
}

/// A receiptless directory of old-generation bytes is still provably ours:
/// install repairs it in place — new entrypoint bytes, a fresh receipt,
/// `core.hooksPath` untouched — and uninstall takes it back whole.
#[test]
#[allow(clippy::unwrap_used)]
fn a_receiptless_old_generation_directory_is_ours_by_content() {
    let w = world();
    let hooks_dir = old_generation_install(&w.repo, false);

    githooks::install(&w.env, &w.repo).unwrap();
    assert!(hooks_dir.join("receipt.json").is_file(), "repaired");
    for hook in githooks::HOOKS {
        assert_eq!(
            std::fs::read_to_string(hooks_dir.join(hook)).unwrap(),
            githooks::entrypoint(hook)
        );
    }
    assert_eq!(
        config_value(&w.repo, "core.hooksPath").as_deref(),
        Some(hooks_dir.display().to_string().as_str())
    );
    assert!(
        !w.repo.join(".git/kendex-hooks").exists(),
        "the live old directory is worked in place, never doubled"
    );

    std::fs::remove_file(hooks_dir.join("receipt.json")).unwrap();
    for hook in githooks::HOOKS {
        std::fs::write(hooks_dir.join(hook), githooks::old_entrypoint(hook)).unwrap();
    }
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
}

/// `core.hooksPath` names the directory git actually executes: a stray
/// empty `kendex-hooks` beside an armed `vstack-hooks` must not shadow
/// it. Repair rewrites the armed directory and says which one; uninstall
/// takes the armed install back, stray untouched.
#[test]
#[allow(clippy::unwrap_used)]
fn a_stray_new_directory_never_shadows_the_armed_old_install() {
    let w = world();
    let hooks_dir = old_generation_install(&w.repo, true);
    let stray = w.repo.join(".git/kendex-hooks");
    std::fs::create_dir_all(&stray).unwrap();

    let report = githooks::repair(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("vstack-hooks")),
        "repair names the directory it rewrote: {report:?}"
    );
    for hook in githooks::HOOKS {
        assert_eq!(
            std::fs::read_to_string(hooks_dir.join(hook)).unwrap(),
            githooks::entrypoint(hook),
            "the armed directory was rewritten, not the stray"
        );
    }
    assert_eq!(
        std::fs::read_dir(&stray).unwrap().count(),
        0,
        "nothing was written into the stray"
    );

    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        !hooks_dir.exists(),
        "the armed install is removed: {report:?}"
    );
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);
    assert!(stray.exists(), "the stray is not kendex's to remove here");
}

/// A repo whose old-named directory was deleted by hand still carries
/// our `core.hooksPath` value. Install re-arms under the name the config
/// still points at instead of refusing to "hijack" our own leftover.
#[test]
#[allow(clippy::unwrap_used)]
fn install_rearms_when_the_config_named_directory_is_gone() {
    let w = world();
    let hooks_dir = old_generation_install(&w.repo, true);
    std::fs::remove_dir_all(&hooks_dir).unwrap();

    githooks::install(&w.env, &w.repo).unwrap();
    assert!(hooks_dir.join("receipt.json").is_file(), "re-armed");
    for hook in githooks::HOOKS {
        assert_eq!(
            std::fs::read_to_string(hooks_dir.join(hook)).unwrap(),
            githooks::entrypoint(hook)
        );
    }
    assert_eq!(
        config_value(&w.repo, "core.hooksPath").as_deref(),
        Some(hooks_dir.display().to_string().as_str()),
        "the config keeps naming the directory it always named"
    );
}

/// The same half-removed state at uninstall: the value is ours by name
/// even with its directory gone, so uninstall clears it — leaving it
/// would make git run no hooks at all.
#[test]
#[allow(clippy::unwrap_used)]
fn uninstall_unsets_the_old_name_when_its_directory_is_gone() {
    let w = world();
    let hooks_dir = old_generation_install(&w.repo, false);
    std::fs::remove_dir_all(&hooks_dir).unwrap();

    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("unset")),
        "{report:?}"
    );
    assert_eq!(config_value(&w.repo, "core.hooksPath"), None);
}

/// The receipt is a claim and the bytes are the proof: an entrypoint
/// someone edited is their hook now, and one such file refuses the whole
/// repair — nothing is rewritten, nothing overwritten.
#[test]
#[allow(clippy::unwrap_used)]
fn repair_refuses_a_hand_edited_entrypoint_and_rewrites_nothing() {
    let w = world();
    let hooks_dir = old_generation_install(&w.repo, true);
    let edited = "#!/bin/sh\nexec my-linter \"$1\"\n";
    std::fs::write(hooks_dir.join("commit-msg"), edited).unwrap();

    let error = githooks::repair(&w.env, &w.repo).unwrap_err();
    assert!(
        error.to_string().contains("commit-msg"),
        "the refusal names the file: {error}"
    );
    assert_eq!(
        std::fs::read_to_string(hooks_dir.join("pre-commit")).unwrap(),
        githooks::old_entrypoint("pre-commit"),
        "a refusal is the whole response — no partial rewrite"
    );
    assert_eq!(
        std::fs::read_to_string(hooks_dir.join("commit-msg")).unwrap(),
        edited
    );
}

/// After a move the receipt's absolute paths are stale. Repair trusts
/// the receipt, so it refuses instead of rewriting under a claim that
/// does not match the live directory — install re-records ownership.
#[test]
#[allow(clippy::unwrap_used)]
fn repair_refuses_when_the_receipt_and_the_live_directory_disagree() {
    let w = world();
    githooks::install(&w.env, &w.repo).unwrap();
    let moved = w.repo.parent().unwrap().join("moved");
    std::fs::rename(&w.repo, &moved).unwrap();

    let error = githooks::repair(&w.env, &moved).unwrap_err();
    assert!(error.to_string().contains("guard install"), "{error}");
}

/// A foreign file among old-generation bytes breaks the by-content proof
/// exactly as it does among new ones: install refuses, and uninstall
/// leaves the directory in place, naming the file.
#[test]
#[allow(clippy::unwrap_used)]
fn a_foreign_file_among_old_generation_bytes_still_refuses() {
    let w = world();
    let hooks_dir = old_generation_install(&w.repo, false);
    std::fs::write(hooks_dir.join("theirs"), "#!/bin/sh\n").unwrap();

    let error = githooks::install(&w.env, &w.repo).unwrap_err();
    assert!(error.to_string().contains("theirs"), "{error}");

    let report = githooks::uninstall(&w.env, &w.repo).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("theirs")),
        "{report:?}"
    );
    assert!(hooks_dir.join("theirs").is_file(), "left in place");
}
