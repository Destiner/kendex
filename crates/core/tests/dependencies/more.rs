//! Optional selections, harness hold-backs, removal preconditions,
//! and preview-first refresh.

use super::*;

/// An optional dependency is installed when it is chosen and not otherwise,
/// and the choice — not what it pulled in — is what the manifest keeps.
#[test]
#[allow(clippy::unwrap_used)]
fn an_optional_dependency_installs_only_once_it_is_chosen() {
    let f = fixture("[skills.dev]\nsource = \"cat\"\n");
    skill(
        &f.source,
        "dev",
        "dependencies:\n  required: [github]\n  optional: [linear]\n",
    );
    skill(&f.source, "linear", "");
    apply_now(&f);
    assert!(!installed(&f, "linear"));

    fs::write(
        f.project.join("vstack.toml"),
        format!(
            "{}\n[optional-dependencies]\ndev = [\"linear\"]\n",
            fs::read_to_string(f.project.join("vstack.toml")).unwrap()
        ),
    )
    .unwrap();
    let report = plan_refresh(&f.env, &f.scope).unwrap();
    assert!(
        report
            .set_changes
            .iter()
            .any(|c| c.name == "linear" && c.direction == SetDirection::Add)
    );
    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(installed(&f, "linear"));

    // The choice survives a refresh, and the item it brought in is recorded
    // as required, never as something the user asked for.
    let report = plan_refresh(&f.env, &f.scope).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(installed(&f, "linear"));
    assert!(
        !lock_of(&f).entries["skill:linear:claude"]
            .reasons
            .contains(&Reason::Requested)
    );
}

/// A dependency its own declaration keeps off a tool is honored there, and
/// the item that needs it is told which tool will run without it.
#[test]
#[allow(clippy::unwrap_used)]
fn a_dependency_held_back_from_a_tool_warns_the_item_that_needs_it() {
    let f = fixture(
        "[skills.dev]\nsource = \"cat\"\nharnesses = [\"claude\", \"codex\"]\n\n[skills.github]\nsource = \"cat\"\nharnesses = [\"claude\"]\n",
    );
    let report = audit(&f.env, &f.scope).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();

    let lock = lock_of(&f);
    assert!(lock.entries.contains_key("skill:dev:codex"));
    assert!(!lock.entries.contains_key("skill:github:codex"));
    assert!(
        report.warnings.iter().any(|w| {
            w.name == "dev" && w.message.contains("Codex") && w.message.contains("github")
        }),
        "{:?}",
        report.warnings
    );
}

/// A removal binds to what the preview showed (invariant 7): content edited
/// in between is never moved to the trash on the strength of a stale plan.
#[test]
#[allow(clippy::unwrap_used)]
fn a_file_changed_after_the_preview_aborts_its_removal() {
    let f = fixture("[skills.dev]\nsource = \"cat\"\nmethod = \"copy\"\n");
    apply_now(&f);
    let skill_md = f.project.join(".claude/skills/dev/SKILL.md");
    assert!(skill_md.is_file());

    let report = ops::remove(&f.env, &f.scope, &["dev".to_owned()], true).unwrap();
    fs::write(&skill_md, "edited after the preview\n").unwrap();
    let error = apply::execute(&f.env, &report.plan, None).unwrap_err();
    assert!(
        matches!(error, vstack_core::error::CoreError::RolledBack { .. }),
        "{error:?}"
    );
    assert_eq!(
        fs::read_to_string(&skill_md).unwrap(),
        "edited after the preview\n"
    );
}

/// Refresh sees the closure move in both directions, and says so before
/// anything is written.
#[test]
#[allow(clippy::unwrap_used)]
fn refresh_previews_what_upstream_added_and_took_away() {
    let f = fixture("[skills.dev]\nsource = \"cat\"\n");
    apply_now(&f);
    assert!(
        plan_refresh(&f.env, &f.scope)
            .unwrap()
            .set_changes
            .is_empty()
    );

    skill(
        &f.source,
        "dev",
        "dependencies:\n  required: [github, worktree]\n",
    );
    skill(&f.source, "worktree", "");
    let added = plan_refresh(&f.env, &f.scope).unwrap();
    assert_eq!(added.set_changes.len(), 1);
    assert_eq!(added.set_changes[0].name, "worktree");
    assert_eq!(added.set_changes[0].direction, SetDirection::Add);
    assert!(added.set_changes[0].reason.contains("required by"));
    apply::execute(&f.env, &added.plan, None).unwrap();

    skill(&f.source, "dev", "dependencies:\n  required: [github]\n");
    let dropped = plan_refresh(&f.env, &f.scope).unwrap();
    assert_eq!(dropped.set_changes.len(), 1);
    assert_eq!(dropped.set_changes[0].name, "worktree");
    assert_eq!(dropped.set_changes[0].direction, SetDirection::Remove);
    apply::execute(&f.env, &dropped.plan, None).unwrap();
    assert!(!installed(&f, "worktree"));
    assert!(installed(&f, "dev") && installed(&f, "github"));
}

/// A catalog that cannot be read this pass knows nothing about what needs
/// what, so it must not be the reason anything is uninstalled.
#[test]
#[allow(clippy::unwrap_used)]
fn an_unreadable_catalog_never_sweeps_a_dependency() {
    let f = fixture("[skills.dev]\nsource = \"cat\"\n");
    apply_now(&f);
    assert!(installed(&f, "github"));

    fs::rename(&f.source, f.source.with_extension("moved")).unwrap();
    let report = plan_refresh(&f.env, &f.scope).unwrap();
    assert!(report.set_changes.is_empty(), "{:?}", report.set_changes);
    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(installed(&f, "github") && installed(&f, "dev"));
}
