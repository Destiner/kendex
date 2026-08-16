//! The package page's queries: sealed, capped, and traversal-proof file
//! reads; a deterministic readme; provenance that names the version and
//! the fork.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use vstack_core::apply;
use vstack_core::engine::audit;
use vstack_core::env::{Env, FakeOs};
use vstack_core::error::CoreError;
use vstack_core::manifest;
use vstack_core::model::{ItemKind, Scope};
use vstack_core::package::detail;
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
fn commit(dir: &Path, message: &str) -> String {
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
    let output = Hardened::git(&["rev-parse", "HEAD"], Some(dir))
        .run()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
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
fn install(w: &World) -> String {
    let dir = w.upstream.join("skills/gh");
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: gh\ndescription: github flows\n---\nBody.\n",
    )
    .unwrap();
    fs::write(dir.join("readme.md"), "lowercase readme").unwrap();
    fs::write(dir.join("README.md"), "# The readme\n").unwrap();
    fs::write(dir.join("references/deep.md"), "deep file").unwrap();
    let commit = commit(&w.upstream, "one");
    git(&w.upstream, &["tag", "v1"]);
    let path = manifest::manifest_path(&w.env, &w.scope);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!(
            "schema = 3\n\n[sources.cat]\nrepo = \"{REPO}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[skills.gh]\nsource = \"cat\"\n"
        ),
    )
    .unwrap();
    let loaded = manifest::load_for_mutation(&path).unwrap().unwrap();
    remote::sync_sources(&w.env, &loaded).unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();
    commit
}

#[test]
#[allow(clippy::unwrap_used)]
fn files_list_sorted_with_the_readme_marked() {
    let w = world();
    install(&w);
    let files = detail::package_files(&w.env, &w.scope, ItemKind::Skill, "gh").unwrap();
    let paths: Vec<(&str, bool)> = files
        .iter()
        .map(|f| (f.path.as_str(), f.is_readme))
        .collect();
    assert_eq!(
        paths,
        vec![
            ("README.md", true),
            ("SKILL.md", false),
            ("readme.md", true),
            ("references/deep.md", false),
        ]
    );
    assert!(files.iter().all(|f| f.size > 0));
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_file_reads_capped_and_traversal_is_refused() {
    let w = world();
    install(&w);
    let file = detail::package_file(
        &w.env,
        &w.scope,
        ItemKind::Skill,
        "gh",
        "references/deep.md",
    )
    .unwrap();
    assert_eq!(file.content, "deep file");
    assert!(!file.truncated);

    for bad in ["../../../etc/passwd", "/etc/passwd", ""] {
        let error = detail::package_file(&w.env, &w.scope, ItemKind::Skill, "gh", bad).unwrap_err();
        assert!(
            matches!(error, CoreError::SourceEscape { .. }),
            "{bad}: {error}"
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn the_exact_readme_wins_over_case_variants() {
    let w = world();
    install(&w);
    let readme = detail::package_readme(&w.env, &w.scope, ItemKind::Skill, "gh")
        .unwrap()
        .unwrap();
    assert_eq!(readme.content, "# The readme\n");
}

#[test]
#[allow(clippy::unwrap_used)]
fn meta_names_the_version_the_link_and_the_fork() {
    let w = world();
    let commit = install(&w);
    let meta = detail::package_meta(&w.env, &w.scope, ItemKind::Skill, "gh").unwrap();
    assert_eq!(meta.source, "cat");
    assert_eq!(meta.repo.as_deref(), Some(REPO));
    assert_eq!(
        meta.repo_url.as_deref(),
        Some("https://github.com/owner/catalog")
    );
    let current = meta.current.unwrap();
    assert_eq!(current.commit, commit);
    assert_eq!(current.label.as_deref(), Some("v1"));
    assert!(meta.installed_at.is_some());
    assert!(meta.fork.is_none());
    assert!(meta.enabled);

    // Fork it: meta says so, and the source flips to local.
    fs::write(
        w.home.join("app/.agents/skills/gh/SKILL.md"),
        "---\nname: gh\ndescription: mine\n---\nMine.\n",
    )
    .unwrap();
    let plan = vstack_core::engine::fork::fork(
        &w.env,
        &w.scope,
        ItemKind::Skill,
        "gh",
        vstack_core::model::HarnessId::Claude,
    )
    .unwrap();
    apply::execute(&w.env, &plan, None).unwrap();
    let meta = detail::package_meta(&w.env, &w.scope, ItemKind::Skill, "gh").unwrap();
    assert_eq!(meta.source, "local");
    let fork = meta.fork.unwrap();
    assert_eq!(fork.source, "cat");
    assert_eq!(fork.repo.as_deref(), Some(REPO));
    assert_eq!(fork.commit.as_deref(), Some(commit.as_str()));
}
