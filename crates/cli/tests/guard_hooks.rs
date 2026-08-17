//! Real commits through the installed hooks: every tool walks through the
//! guards — a plain `git commit`, no harness anywhere — the chain execs the
//! hook git itself would have run, the commit-msg lane takes the relative
//! message path git passes, and the guard judges the exact index git names
//! in GIT_INDEX_FILE (a partial commit's temporary index, not the staged
//! one).
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[allow(clippy::expect_used)]
fn run(home: &Path, cwd: &Path, program: &str, args: &[&str]) -> Output {
    let bin_dir = PathBuf::from(env!("CARGO_BIN_EXE_vstack"))
        .parent()
        .expect("binary has a parent")
        .to_path_buf();
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin_dir.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("process runs")
}

#[allow(clippy::unwrap_used)]
fn git(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    run(home, cwd, "git", args)
}

#[allow(clippy::unwrap_used)]
fn git_ok(home: &Path, cwd: &Path, args: &[&str]) {
    let output = git(home, cwd, args);
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[allow(clippy::unwrap_used)]
fn repo(home: &Path) -> PathBuf {
    let root = home.join("proj");
    std::fs::create_dir_all(&root).unwrap();
    git_ok(home, &root, &["init", "--quiet", "-b", "main"]);
    git_ok(home, &root, &["config", "user.email", "t@t"]);
    git_ok(home, &root, &["config", "user.name", "t"]);
    std::fs::write(root.join("a.txt"), "hi\n").unwrap();
    git_ok(home, &root, &["add", "-A"]);
    git_ok(home, &root, &["commit", "--quiet", "-m", "feat: base"]);
    root
}

#[test]
#[allow(clippy::unwrap_used)]
fn commits_walk_through_the_guards_whatever_tool_makes_them() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let root = repo(home);
    let install = run(home, &root, "vstack", &["guard", "install"]);
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );

    // A staged work marker blocks the commit; the message names the check.
    std::fs::write(
        root.join("src.rs"),
        format!("// {}{}: later\nfn main() {{}}\n", "TO", "DO"),
    )
    .unwrap();
    git_ok(home, &root, &["add", "-A"]);
    let blocked = git(home, &root, &["commit", "-m", "feat: add src"]);
    assert!(!blocked.status.success(), "the commit must be blocked");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&blocked.stdout),
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(said.contains("todo-ban"), "{said}");

    // Fixed and staged, the same commit goes through — and the commit-msg
    // lane (relative COMMIT_EDITMSG path) accepts the conventional header.
    std::fs::write(root.join("src.rs"), "fn main() {}\n").unwrap();
    git_ok(home, &root, &["add", "-A"]);
    git_ok(home, &root, &["commit", "--quiet", "-m", "feat: add src"]);

    // A non-conventional message is blocked by the commit-msg lane.
    std::fs::write(root.join("b.txt"), "b\n").unwrap();
    git_ok(home, &root, &["add", "-A"]);
    let bad = git(home, &root, &["commit", "-m", "added some stuff"]);
    assert!(!bad.status.success());
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
    assert!(said.contains("commit-msg"), "{said}");
    git_ok(home, &root, &["commit", "--quiet", "-m", "feat: add b"]);
}

#[test]
#[allow(clippy::unwrap_used)]
fn the_chain_execs_the_hook_git_itself_would_have_run() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let root = repo(home);
    run(home, &root, "vstack", &["guard", "install"]);

    // A clean pre-existing hook in git's own directory still runs — and
    // its failure decides.
    let own_hooks = root.join(".git/hooks");
    std::fs::create_dir_all(&own_hooks).unwrap();
    let marker = home.join("chained-ran");
    std::fs::write(
        own_hooks.join("pre-commit"),
        format!("#!/bin/sh\ntouch {}\nexit 1\n", marker.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        own_hooks.join("pre-commit"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    std::fs::write(root.join("c.txt"), "c\n").unwrap();
    git_ok(home, &root, &["add", "-A"]);
    let blocked = git(home, &root, &["commit", "-m", "feat: add c"]);
    assert!(
        marker.exists(),
        "the foreign hook ran after the guards passed"
    );
    assert!(!blocked.status.success(), "its exit status decides");

    std::fs::write(own_hooks.join("pre-commit"), "#!/bin/sh\nexit 0\n").unwrap();
    git_ok(home, &root, &["commit", "--quiet", "-m", "feat: add c"]);
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_partial_commit_is_judged_on_the_index_git_names() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let root = repo(home);
    run(home, &root, "vstack", &["guard", "install"]);

    // The staged index carries a work marker in b.rs; the partial commit
    // does not include it. Judging the staged index would block a commit
    // that carries no marker — GIT_INDEX_FILE names the honest one.
    std::fs::write(
        root.join("b.rs"),
        format!("// {}{}: staged but not committed\n", "TO", "DO"),
    )
    .unwrap();
    std::fs::write(root.join("a.txt"), "hi there\n").unwrap();
    git_ok(home, &root, &["add", "-A"]);
    let partial = git(
        home,
        &root,
        &["commit", "--quiet", "-m", "feat: a only", "--", "a.txt"],
    );
    assert!(
        partial.status.success(),
        "the temporary index is what is judged: {}{}",
        String::from_utf8_lossy(&partial.stdout),
        String::from_utf8_lossy(&partial.stderr)
    );

    // And the marker still blocks the commit that would actually carry it.
    let full = git(home, &root, &["commit", "-m", "feat: the rest"]);
    assert!(!full.status.success());
}
