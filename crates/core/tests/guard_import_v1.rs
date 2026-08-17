//! Legacy v1 guard settings: the refusal that names the conversion, the
//! one-time import, and the differential test holding the legacy-glob
//! dialect to /bin/sh's own case semantics over a synthetic corpus.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use vstack_core::guard::{self, GuardCtx, patterns, settings::Policy};
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

#[test]
#[allow(clippy::unwrap_used)]
fn legacy_settings_without_guards_tables_refuse_naming_the_conversion() {
    let r = repo();
    stage(&r, "src/a.rs", "fn main() {}\n");
    stage(
        &r,
        "vstack.settings.toml",
        "[env]\nSIZE_RATCHET_THRESHOLD = \"300\"\n",
    );
    let error = Policy::load(&ctx(&r), "test").unwrap_err();
    assert!(error.to_string().contains("import-v1"), "{error}");
}

#[test]
#[allow(clippy::unwrap_used)]
fn import_v1_converts_settings_and_marks_excludes_imported() {
    let r = repo();
    stage(
        &r,
        "vstack.settings.toml",
        "[env]\nSIZE_RATCHET_THRESHOLD = \"300\"\nSIZE_RATCHET_CLASSES = \"*.ts=250;ui/*=200\"\nGROWTH_GUARDS_BYTE_CEILING_KB = \"150\"\nGROWTH_GUARDS_COMMIT_TYPES = \"feat fix docs\"\nGROWTH_GUARDS_PRE_COMMIT_LOCAL = \"tools/check\"\n",
    );
    stage(
        &r,
        "tools/size-ratchet-excludes",
        "target/*\tbuild output\n",
    );
    let report = guard::import::run(&ctx(&r)).unwrap();
    assert!(report.changed);
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("must never name the executable")),
        "the machine-local extension point is never converted from a repo file: {:?}",
        report.lines
    );

    let text = std::fs::read_to_string(r.root.join("vstack.settings.toml")).unwrap();
    assert!(text.contains("[guards.size-ratchet]"), "{text}");
    assert!(text.contains("threshold = 300"), "{text}");
    assert!(text.contains("classes-dialect = \"legacy-glob\""), "{text}");
    assert!(text.contains("ceiling-kb = 150"), "{text}");
    assert!(!text.contains("pre-commit-local"), "{text}");
    let excludes = std::fs::read_to_string(r.root.join("tools/size-ratchet-excludes")).unwrap();
    assert!(
        excludes.starts_with(patterns::LEGACY_DIALECT_MARKER),
        "imported excludes keep their dialect, marked: {excludes}"
    );

    // Converted and staged, the guards read the new tables.
    git(&r.root, &["add", "-A"]);
    let policy = Policy::load(&ctx(&r), "test").unwrap();
    assert_eq!(
        policy
            .positive_int("size-ratchet", "threshold", 400)
            .unwrap(),
        300
    );
    assert_eq!(
        policy.string_list("commit-msg", "types", &[]).unwrap(),
        ["feat", "fix", "docs"]
    );
}

/// The differential test settled decision 7 asks for: the legacy-glob
/// dialect against v1's own matcher — `/bin/sh` case globbing — over a
/// synthetic corpus, not just today's trees.
#[test]
#[allow(clippy::unwrap_used)]
fn legacy_glob_matches_sh_case_semantics_over_a_corpus() {
    let patterns_corpus = [
        "target/*",
        "*.lock",
        "src/*.rs",
        "a?c",
        "v[12]/x",
        "v[!12]/x",
        "docs/*.md",
        "*",
        "deep/*/nested",
        "[a-c]x",
    ];
    let paths_corpus = [
        "target/a/b/c.rs",
        "target/c.rs",
        "Cargo.lock",
        "sub/Cargo.lock",
        "src/main.rs",
        "src/deep/main.rs",
        "abc",
        "a/c",
        "v1/x",
        "v3/x",
        "docs/readme.md",
        "docs/sub/readme.md",
        "deep/one/nested",
        "deep/one/two/nested",
        "ax",
        "dx",
    ];
    for pattern in patterns_corpus {
        for path in paths_corpus {
            let sh = std::process::Command::new("/bin/sh")
                .args([
                    "-c",
                    r#"case "$2" in $1) exit 0;; esac; exit 1"#,
                    "sh",
                    pattern,
                    path,
                ])
                .status()
                .unwrap()
                .success();
            let ours = patterns::matches(pattern, path, patterns::Dialect::LegacyGlob);
            assert_eq!(
                ours, sh,
                "dialect disagreement: pattern '{pattern}' vs path '{path}' (sh={sh})"
            );
        }
    }
}
