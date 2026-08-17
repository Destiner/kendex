//! The registry the Settings page renders promises every decision on the
//! machine. A scope it cannot read is reported as such beside the rest —
//! never silently skipped, which would show a shorter list with no hint
//! that anything is missing.
#![cfg(unix)]

use std::fs;

use vstack_app::audit::ScopeErrorKind;
use vstack_app::decisions::decisions_view;
use vstack_core::env::{Env, FakeOs};

#[test]
#[allow(clippy::unwrap_used)]
fn an_unreadable_scope_is_reported_beside_the_decisions_it_hides() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join("vstack.toml"),
        "schema = 5\n[nonsense]\nx = 1\n",
    )
    .unwrap();
    let mut settings = vstack_core::settings::load(&env).unwrap();
    settings.projects.push(project);
    vstack_core::settings::save(&env, &settings).unwrap();

    let view = decisions_view(&env).unwrap();
    assert!(view.decisions.is_empty());
    assert_eq!(view.errors.len(), 1, "{:?}", view.errors.len());
    assert!(matches!(
        view.errors[0].error.kind,
        ScopeErrorKind::ManifestInvalid
    ));
}
