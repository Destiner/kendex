//! What decides a guard verdict, and what cannot: an untracked policy
//! file on disk is not policy, the commit-msg hook lane honors its
//! enabled switch, a path the baseline cannot carry is curable by the
//! excludes the refusal names, and the lanes report hits per line.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use vstack_core::guard::{self, GuardCtx, settings::Policy, size_ratchet};
use vstack_core::process::Hardened;

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

fn todo_marker() -> String {
    format!("// {}{}: finish this\n", "TO", "DO")
}

/// The one machine-local override is the environment. A settings or
/// excludes file that sits on disk but not in the index — however it got
/// there — decides nothing: the commit does not carry it.
#[test]
#[allow(clippy::unwrap_used)]
fn an_untracked_policy_file_on_disk_is_not_policy() {
    let r = repo();
    stage(&r, "src/lib.rs", &todo_marker());
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(out.violations, 1, "{:?}", out.lines);

    // An unstaged excludes file naming everything.
    std::fs::create_dir_all(r.root.join("tools")).unwrap();
    std::fs::write(r.root.join("tools/todo-ban-excludes"), "**\tnope\n").unwrap();
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(
        out.violations, 1,
        "an untracked excludes file changes nothing: {:?}",
        out.lines
    );

    // An unstaged settings file switching the check off.
    std::fs::create_dir_all(r.root.join(".vstack")).unwrap();
    std::fs::write(
        r.root.join(".vstack/settings.toml"),
        "[guards.todo-ban]\nenabled = false\n",
    )
    .unwrap();
    assert!(
        policy(&r).enabled("todo-ban").unwrap(),
        "an untracked settings file changes nothing"
    );

    // Staged, the same file governs.
    git(&r.root, &["add", "--", ".vstack/settings.toml"]);
    assert!(!policy(&r).enabled("todo-ban").unwrap());
}

#[test]
#[allow(clippy::unwrap_used)]
fn the_commit_msg_hook_lane_honors_its_enabled_switch() {
    let r = repo();
    stage(&r, "a.txt", "a\n");
    let report = guard::run_commit_msg(&ctx(&r), "not conventional at all\n");
    assert_eq!(report.exit_code(), 1, "{:?}", report.lines);
    let report = guard::run_commit_msg(&ctx(&r), "feat: fine\n");
    assert_eq!(report.exit_code(), 0, "{:?}", report.lines);

    stage(
        &r,
        "vstack.settings.toml",
        "[guards.commit-msg]\nenabled = false\n",
    );
    let report = guard::run_commit_msg(&ctx(&r), "not conventional at all\n");
    assert_eq!(report.exit_code(), 0, "{:?}", report.lines);
    assert!(
        report.lines.iter().any(|l| l.contains("disabled")),
        "{:?}",
        report.lines
    );
    // The standalone verb still judges: the switch is the hook's.
    let out = guard::commit_msg::run(&policy(&r), "not conventional at all\n").unwrap();
    assert_eq!(out.violations, 1);
}

/// The refusal for a path the baseline TSV cannot carry names its remedy
/// — an excludes row — so the excludes must apply before the refusal. A
/// newline in a path parses (records are NUL-delimited) and is refused
/// the same way.
#[test]
#[allow(clippy::unwrap_used)]
fn a_tab_or_newline_in_a_tracked_path_is_curable_by_the_excludes_it_names() {
    let r = repo();
    stage(&r, "src/ok.rs", "fn main() {}\n");
    stage(&r, "assets/a\tb.txt", "x\n");
    stage(&r, "assets/c\nd.txt", "y\n");
    let error = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap_err();
    assert!(error.to_string().contains("excludes row"), "{error}");
    stage(&r, "tools/size-ratchet-excludes", "assets/**\tgenerated\n");
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
    let out = guard::suppression_ban::run(&ctx(&r), &policy(&r), false).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
}

/// One pass per lane: every hit is reported with its file, line, and
/// text, and a path carrying `:` cannot garble the report.
#[test]
#[allow(clippy::unwrap_used)]
fn a_lane_reports_every_hit_by_file_and_line() {
    let r = repo();
    stage(
        &r,
        "src/a:b.rs",
        &format!(
            "fn a() {{}}\n{}fn b() {{}}\n{}",
            todo_marker(),
            todo_marker()
        ),
    );
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert_eq!(out.violations, 2, "{:?}", out.lines);
    assert!(
        out.lines.iter().any(|l| l.contains("src/a:b.rs:2:")),
        "{:?}",
        out.lines
    );
    assert!(
        out.lines.iter().any(|l| l.contains("src/a:b.rs:4:")),
        "{:?}",
        out.lines
    );
}

/// The guards parse git's output; a user's color settings must never
/// reach it — an escape sequence around a path is not a path.
#[test]
#[allow(clippy::unwrap_used)]
fn a_users_color_config_cannot_garble_what_the_guards_parse() {
    let r = repo();
    git(&r.root, &["config", "color.ui", "always"]);
    git(&r.root, &["config", "color.grep", "always"]);
    let long: String = (0..30).map(|i| format!("line {i}\n")).collect();
    stage(&r, "src/long.rs", &long);
    stage(&r, "tools/size-ratchet-baseline.tsv", "src/long.rs\t30\n");
    stage(
        &r,
        "vstack.settings.toml",
        "[guards.size-ratchet]\nthreshold = 20\n",
    );
    let out = size_ratchet::run(&ctx(&r), &policy(&r), size_ratchet::Mode::Check).unwrap();
    assert_eq!(out.violations, 0, "{:?}", out.lines);
    stage(&r, "src/lib.rs", &todo_marker());
    let out = guard::todo_ban(&ctx(&r), &policy(&r)).unwrap();
    assert!(
        out.lines.iter().any(|l| l.contains("src/lib.rs:1:")),
        "{:?}",
        out.lines
    );
}
