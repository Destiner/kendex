//! Unsubscribe — remove: the closure of a source is what leaves with it
//! (declared items and their derived dependencies), computed by re-expansion,
//! and removing the source uninstalls exactly that.
#![cfg(unix)]

use std::fs;
use std::path::Path;

use kendex_core::engine::detach;
use kendex_core::env::{Env, FakeOs};
use kendex_core::manifest::{self, ManifestFile};
use kendex_core::model::Scope;
use kendex_core::{apply, source_ops};

#[allow(clippy::unwrap_used)]
fn skill(catalog: &Path, name: &str, body: &str) {
    let dir = catalog.join("skills").join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {name}\n---\n{body}\n"),
    )
    .unwrap();
}

#[allow(clippy::unwrap_used)]
fn world(
    declarations: &str,
    extra_sources: &str,
) -> (tempfile::TempDir, Env, Scope, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let env = Env::fake(home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    let catalog = home.join("catalog");
    fs::write(
        project.join("kendex.toml"),
        format!(
            "schema = 5\n\n[sources.cat]\npath = \"{}\"\n{extra_sources}\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n{declarations}",
            catalog.display()
        ),
    )
    .unwrap();
    (tmp, env, Scope::Project { root: project }, catalog)
}

#[allow(clippy::unwrap_used)]
fn apply_now(env: &Env, scope: &Scope) {
    let report = kendex_core::engine::audit(env, scope).unwrap();
    apply::execute(env, &report.plan, None).unwrap();
}

#[allow(clippy::unwrap_used)]
fn manifest_of(env: &Env, scope: &Scope) -> manifest::Manifest {
    match manifest::load(&manifest::manifest_path(env, scope)).unwrap() {
        ManifestFile::Current(m) => *m,
        other => panic!("expected current manifest, got {other:?}"),
    }
}

/// A skill that requires another (a derived dependency) is part of the closure
/// even though the dependency never names the source in the manifest.
#[test]
#[allow(clippy::unwrap_used)]
fn removing_a_source_takes_its_closure_including_derived_deps() {
    let (_tmp, env, scope, catalog) = world("[skills.gh]\nsource = \"cat\"\n", "");
    // gh declares a required dependency on `common`.
    skill(
        &catalog,
        "gh",
        "---\ndependencies:\n  required: [common]\n---\nbody",
    );
    // Re-write gh's SKILL.md with the dependency frontmatter (skill() wrote a
    // plain one first; overwrite it).
    fs::write(
        catalog.join("skills/gh/SKILL.md"),
        "---\nname: gh\ndescription: gh\ndependencies:\n  required: [common]\n---\nbody\n",
    )
    .unwrap();
    skill(&catalog, "common", "shared");
    apply_now(&env, &scope);
    assert!(scope_skill(&scope, "gh").exists());
    assert!(scope_skill(&scope, "common").exists());

    // The closure names both, and marks `common` as derived.
    let closure = detach::closure(&env, &scope, "cat", &manifest_of(&env, &scope)).unwrap();
    let names: Vec<&str> = closure.items.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"gh"), "{names:?}");
    assert!(names.contains(&"common"), "{names:?}");
    assert!(
        closure
            .items
            .iter()
            .any(|i| i.name == "common" && i.derived),
        "the dependency is derived, not declared"
    );

    // Remove uninstalls the whole closure and drops the source.
    let report = detach::remove(&env, &scope, "cat").unwrap();
    apply::execute(&env, &report.plan, None).unwrap();
    assert!(!scope_skill(&scope, "gh").exists());
    assert!(!scope_skill(&scope, "common").exists());
    assert!(!manifest_of(&env, &scope).sources.contains_key("cat"));
}

fn scope_skill(scope: &Scope, name: &str) -> std::path::PathBuf {
    let Scope::Project { root } = scope else {
        unreachable!()
    };
    root.join(".claude/skills").join(name)
}

/// Removing a source refuses while it cannot be read: a closure inferred from
/// an unreachable catalog could strand or over-sweep a derived dependency.
#[test]
#[allow(clippy::unwrap_used)]
fn removing_an_unreachable_source_refuses() {
    let (_tmp, env, scope, catalog) = world("[skills.gh]\nsource = \"cat\"\n", "");
    skill(&catalog, "gh", "body");
    apply_now(&env, &scope);
    // Make the catalog unreadable by removing it.
    fs::remove_dir_all(&catalog).unwrap();
    assert!(detach::remove(&env, &scope, "cat").is_err());
    // The subscription is untouched by the refusal.
    assert!(manifest_of(&env, &scope).sources.contains_key("cat"));
}

/// The plain "nothing installed" case still works through the ordinary source
/// removal path — a subscription with no installations just drops.
#[test]
#[allow(clippy::unwrap_used)]
fn removing_an_empty_subscription_drops_it() {
    let (_tmp, env, scope, catalog) = world("", "");
    skill(&catalog, "gh", "body");
    let report = source_ops::remove_source(&env, &scope, "cat").unwrap();
    apply::execute(&env, &report.plan, None).unwrap();
    assert!(!manifest_of(&env, &scope).sources.contains_key("cat"));
}
