//! The guard family against crafted repositories: each check red and green,
//! policy read from the index (the two fail-opens closed), unjudgeable
//! index states refused, the ratchets tighten-only, and the legacy-glob
//! dialect matching v1's own matcher over a synthetic corpus.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use kendex_core::guard::{self, GuardCtx, settings::Policy, size_ratchet};
use kendex_core::process::Hardened;

struct Repo {
    _tmp: tempfile::TempDir,
    root: PathBuf,
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
fn repo() -> Repo {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    git(&root, &["init", "--quiet", "-b", "main"]);
    git(&root, &["config", "user.email", "t@t"]);
    git(&root, &["config", "user.name", "t"]);
    Repo { _tmp: tmp, root }
}

#[allow(clippy::unwrap_used)]
fn stage(repo: &Repo, path: &str, content: &str) {
    let target = repo.root.join(path);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, content).unwrap();
    git(&repo.root, &["add", "--", path]);
}

fn ctx(repo: &Repo) -> GuardCtx {
    GuardCtx {
        root: repo.root.clone(),
        index_file: None,
    }
}

#[allow(clippy::unwrap_used)]
fn policy(repo: &Repo) -> Policy {
    Policy::load(&ctx(repo), "test").unwrap()
}

/// The banned shapes, assembled at runtime so this repository's own guard
/// never mistakes the fixtures for real markers.
fn todo_marker() -> String {
    format!("// {}{}: finish this\n", "TO", "DO")
}

fn module_wide_allow() -> String {
    format!("#!{}(dead_code)]\nfn a() {{}}\n", "[allow")
}

fn bare_allow(body: &str) -> String {
    format!("#{}({body})]\n", "[allow")
}

#[test]
#[allow(clippy::unwrap_used)]
fn todo_ban_fires_on_comment_markers_and_spares_prose() {
    let r = repo();
    stage(
        &r,
        "src/lib.rs",
        &format!("{}fn main() {{}}\n", todo_marker()),
    );
    stage(
        &r,
        "docs/notes.md",
        &format!("The word {}{} in prose does not fire.\n", "TO", "DO"),
    );
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);
    assert!(out.lines.iter().any(|l| l.contains("src/lib.rs")));

    // The worktree is not the index: fixing the file without staging the
    // fix changes nothing (the commit still carries the marker)...
    std::fs::write(r.root.join("src/lib.rs"), "fn main() {}\n").unwrap();
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(out.violations, 1, "the index is what is judged");
    // ...and staging it clears the verdict.
    git(&r.root, &["add", "--", "src/lib.rs"]);
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
}

#[test]
#[allow(clippy::unwrap_used)]
fn todo_ban_excludes_need_a_reason_and_then_apply() {
    let r = repo();
    stage(&r, "vendor/x.js", &todo_marker());
    stage(&r, "tools/todo-ban-excludes", "vendor/**\n");
    let error = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap_err();
    assert!(error.to_string().contains("pattern<TAB>reason"), "{error}");

    stage(&r, "tools/todo-ban-excludes", "vendor/**\tvendored tree\n");
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
}

#[test]
#[allow(clippy::unwrap_used)]
fn byte_ceiling_gates_added_blobs_and_exempts_lockfiles() {
    let r = repo();
    let big = "x".repeat(300 * 1024);
    stage(&r, "assets/big.bin", &big);
    stage(&r, "Cargo.lock", &big);
    stage(&r, "src/ok.rs", "fn main() {}\n");
    let out = guard::byte_ceiling::run(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);
    assert!(out.lines.iter().any(|l| l.contains("assets/big.bin")));
    assert!(!out.lines.iter().any(|l| l.contains("Cargo.lock")));

    // A rename of an existing large file is not an addition.
    git(&r.root, &["commit", "--quiet", "-m", "feat: base"]);
    git(&r.root, &["mv", "assets/big.bin", "assets/moved.bin"]);
    let out = guard::byte_ceiling::run(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(
        out.violations, 0,
        "renames are not additions: {:?}",
        out.lines
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn suppression_ban_blanket_fails_flat_and_bare_allows_ratchet() {
    let r = repo();
    stage(&r, "src/a.rs", &module_wide_allow());
    let out = guard::suppression_ban::run(&ctx(&r), &policy(&r), false).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);

    // A per-line allow with a stated reason is legal.
    stage(
        &r,
        "src/a.rs",
        &format!(
            "{}fn a() {{}}\n",
            bare_allow("dead_code, reason = \"wired next commit\"")
        ),
    );
    let out = guard::suppression_ban::run(&ctx(&r), &policy(&r), false).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);

    // A reasonless allow is a new bare allow with no baseline row.
    stage(
        &r,
        "src/b.rs",
        &format!("{}fn b() {{}}\n", bare_allow("dead_code")),
    );
    let out = guard::suppression_ban::run(&ctx(&r), &policy(&r), false).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);
    assert!(out.lines.iter().any(|l| l.contains("new bare allow")));

    // A baseline row freezes it; growth past the row fails.
    stage(&r, "tools/suppression-baseline.tsv", "src/b.rs\t1\n");
    let out = guard::suppression_ban::run(&ctx(&r), &policy(&r), false).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
    stage(
        &r,
        "src/b.rs",
        &format!(
            "{}fn b() {{}}\n{}use std::fmt;\n",
            bare_allow("dead_code"),
            bare_allow("unused_imports")
        ),
    );
    let out = guard::suppression_ban::run(&ctx(&r), &policy(&r), false).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);
    assert!(out.lines.iter().any(|l| l.contains("grew")));
}

#[test]
#[allow(clippy::unwrap_used)]
fn size_ratchet_red_green_with_classes_and_seed_refusal() {
    let r = repo();
    let long: String = (0..30).map(|i| format!("line {i}\n")).collect();
    stage(&r, "src/long.rs", &long);
    stage(&r, "src/short.rs", "fn main() {}\n");
    stage(
        &r,
        "vstack.settings.toml",
        "[guards.size-ratchet]\nthreshold = 20\nclasses = [\n  { pattern = \"src/short*\", threshold = 1 },\n]\n",
    );
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);
    assert!(out.lines.iter().any(|l| l.contains("src/long.rs")));

    // First match wins: a tighter class over the short file fires too.
    stage(&r, "src/short.rs", "fn main() {}\nfn extra() {}\n");
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(
        out.violations, 2,
        "class threshold applies: {:?}",
        out.lines
    );

    // Seed writes the first baseline and re-checks clean; seeding again
    // refuses.
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Seed).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
    git(&r.root, &["add", "-A"]);
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(
        out.violations, 0,
        "seeded baseline freezes reality: {:?}",
        out.lines
    );
    let error = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Seed).unwrap_err();
    assert!(error.to_string().contains("refuses"), "{error}");

    // The ratchet only moves down: shrinking the file makes the row loose.
    stage(
        &r,
        "src/long.rs",
        &long
            .lines()
            .take(25)
            .map(|l| format!("{l}\n"))
            .collect::<String>(),
    );
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);
    assert!(out.lines.iter().any(|l| l.contains("looser than reality")));
    // --update tightens, and the staged copy of the baseline governs the
    // next check.
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Update).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
    git(&r.root, &["add", "-A"]);
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
}

#[test]
#[allow(clippy::unwrap_used)]
fn policy_is_read_from_the_index_not_the_worktree() {
    let r = repo();
    let long: String = (0..30).map(|i| format!("line {i}\n")).collect();
    stage(&r, "src/long.rs", &long);
    stage(
        &r,
        "vstack.settings.toml",
        "[guards.size-ratchet]\nthreshold = 20\n",
    );
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);

    // The exact fail-open v1 closed: a permissive unstaged settings copy
    // must not authorize stricter staged content.
    std::fs::write(
        r.root.join("vstack.settings.toml"),
        "[guards.size-ratchet]\nthreshold = 4000\n",
    )
    .unwrap();
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(
        out.violations, 1,
        "an unstaged settings edit must not change the staged verdict: {:?}",
        out.lines
    );

    // Same for the baseline: an unstaged row bump authorizes nothing.
    stage(&r, "tools/size-ratchet-baseline.tsv", "src/long.rs\t30\n");
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
    stage(&r, "src/long.rs", &format!("{long}more\n"));
    std::fs::write(
        r.root.join("tools/size-ratchet-baseline.tsv"),
        "src/long.rs\t31\n",
    )
    .unwrap();
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(
        out.violations, 1,
        "an unstaged baseline bump must not authorize staged growth: {:?}",
        out.lines
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn unjudgeable_index_states_are_exit_2_never_a_skip() {
    let r = repo();
    stage(&r, "src/a.rs", "fn main() {}\n");
    // Intent-to-add: content exists on disk, none is staged — judging the
    // empty stand-in would wave through exactly this file.
    std::fs::write(r.root.join("later.rs"), todo_marker()).unwrap();
    git(&r.root, &["add", "-N", "--", "later.rs"]);
    let error = Policy::load(&ctx(&r), "test").unwrap_err();
    assert!(error.to_string().contains("intent-to-add"), "{error}");
    git(&r.root, &["add", "--", "later.rs"]);
    assert!(
        Policy::load(&ctx(&r), "test").is_ok(),
        "staged content judges fine"
    );
}
