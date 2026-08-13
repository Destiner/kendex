//! The surface model: a physical surface consumed by several harnesses
//! carries exactly one variant; other surfaces link to it while their bytes
//! match and get their own tree when they diverge.
#![cfg(unix)]

use std::fs;

use vstack_core::apply;
use vstack_core::engine::audit;
use vstack_core::env::{Env, FakeOs};
use vstack_core::model::Scope;

#[test]
#[allow(clippy::unwrap_used)]
fn codex_and_pi_share_one_project_variant_and_claude_links_while_equal() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();

    let source = home.join("catalog");
    fs::create_dir_all(source.join("skills/gh")).unwrap();
    fs::write(
        source.join("skills/gh/SKILL.md"),
        "---\nname: gh\ndescription: github\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        project.join("vstack.toml"),
        format!(
            "schema = 2\n\n[sources.cat]\npath = \"{}\"\n\n[install]\nharnesses = [\"claude\", \"codex\", \"pi\"]\nmethod = \"symlink\"\n\n[skills.gh]\nsource = \"cat\"\n",
            source.display()
        ),
    )
    .unwrap();

    let scope = Scope::Project {
        root: project.clone(),
    };
    let report = audit(&env, &scope).unwrap();
    apply::execute(&env, &report.plan, None).unwrap();

    // Codex and Pi read the same physical tree — one variant, no links.
    let shared = project.join(".agents/skills/gh");
    assert!(shared.join("SKILL.md").is_file());
    assert!(!shared.is_symlink());
    // Claude's variant currently matches, so it deduplicates onto the
    // shared tree through a link rather than a second copy.
    let claude = project.join(".claude/skills/gh");
    assert_eq!(fs::read_link(&claude).unwrap(), shared);

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(project.join(".vstack-lock.json")).unwrap())
            .unwrap();
    for key in ["skill:gh:claude", "skill:gh:codex", "skill:gh:pi"] {
        assert!(lock["entries"].get(key).is_some(), "{key} missing");
    }
    assert!(audit(&env, &scope).unwrap().drift.is_empty());
}
