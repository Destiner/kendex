//! Settings seeding through real applies: a skill that ships
//! `vstack.settings.toml.example` merges its `[env]` defaults into the
//! project's settings file — write-if-absent per key, user edits win.
#![cfg(unix)]

use std::fs;
use std::path::PathBuf;

use vstack_core::apply;
use vstack_core::engine::audit;
use vstack_core::env::{Env, FakeOs};
use vstack_core::model::Scope;

const TEMPLATE: &str =
    "[env]\n# Which reviewers run by default.\nREVIEWERS = \"arch,security\"\n\nDEPTH = \"2\"\n";

struct Fixture {
    _tmp: tempfile::TempDir,
    env: Env,
    scope: Scope,
    project: PathBuf,
}

#[allow(clippy::unwrap_used)]
fn fixture(enabled: bool) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();

    let source = home.join("catalog");
    let skill = source.join("skills/review");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: review\ndescription: review changes\n---\nBody.\n",
    )
    .unwrap();
    fs::write(skill.join("vstack.settings.toml.example"), TEMPLATE).unwrap();

    fs::write(
        project.join("vstack.toml"),
        format!(
            "schema = 5\n\n[sources.cat]\npath = \"{}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[skills.review]\nsource = \"cat\"\nenabled = {enabled}\n",
            source.display()
        ),
    )
    .unwrap();

    Fixture {
        env,
        scope: Scope::Project {
            root: project.clone(),
        },
        project,
        _tmp: tmp,
    }
}

#[allow(clippy::unwrap_used)]
fn apply_now(f: &Fixture) {
    let report = audit(&f.env, &f.scope).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();
}

#[test]
#[allow(clippy::unwrap_used)]
fn seeds_env_defaults_and_never_overwrites_user_values() {
    let f = fixture(true);
    apply_now(&f);

    let settings_path = f.project.join("vstack.settings.toml");
    let seeded = fs::read_to_string(&settings_path).unwrap();
    assert!(seeded.contains("[env]"));
    assert!(seeded.contains("# Which reviewers run by default."));
    assert!(seeded.contains("REVIEWERS = \"arch,security\""));
    assert!(seeded.contains("DEPTH = \"2\""));

    // A user-edited value survives every later apply, wherever it lives.
    let edited = seeded.replace("\"arch,security\"", "\"mine\"");
    fs::write(&settings_path, &edited).unwrap();
    apply_now(&f);
    let after = fs::read_to_string(&settings_path).unwrap();
    assert_eq!(after, edited);

    // A clean pass plans nothing for the settings file.
    let report = audit(&f.env, &f.scope).unwrap();
    assert!(
        !report
            .plan
            .ops
            .iter()
            .any(|op| op.description.starts_with("Seed ")),
        "clean settings file must not be re-planned"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn disabled_skill_does_not_seed() {
    let f = fixture(false);
    apply_now(&f);
    assert!(!f.project.join("vstack.settings.toml").exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn occupied_settings_path_is_a_conflict_not_a_clobber() {
    let f = fixture(true);
    fs::create_dir_all(f.project.join("vstack.settings.toml")).unwrap();
    let report = audit(&f.env, &f.scope).unwrap();
    assert!(
        report
            .drift
            .iter()
            .any(|row| row.detail.contains("not a regular file"))
    );
    assert!(
        !report
            .plan
            .ops
            .iter()
            .any(|op| op.description.starts_with("Seed "))
    );
}
