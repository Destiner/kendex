//! The fork operation over the shared edits-and-forks harness.

use std::fs;

use super::*;

#[test]
#[allow(clippy::unwrap_used)]
fn fork_keeps_the_name_pauses_updates_and_survives_refresh() {
    let w = world();
    write_skill(&w.upstream, "gh", "Upstream.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);

    fs::write(
        skill_file(&w),
        "---\nname: gh\ndescription: mine\n---\nMy fork.\n",
    )
    .unwrap();
    let plan = fork::fork(&w.env, &w.scope, ItemKind::Skill, "gh", HarnessId::Claude).unwrap();
    apply::execute(&w.env, &plan, None).unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();

    // The fork's bytes live in the local source and render under the name.
    assert!(
        fs::read_to_string(w.home.join("app/.vstack-local/skills/gh/SKILL.md"))
            .unwrap()
            .contains("My fork.")
    );
    assert!(
        fs::read_to_string(skill_file(&w))
            .unwrap()
            .contains("My fork.")
    );
    let text = fs::read_to_string(manifest::manifest_path(&w.env, &w.scope)).unwrap();
    assert!(text.contains("[forks.skill.gh]"), "{text}");
    assert!(text.contains("source = \"local\""));

    // Upstream keeps moving; the fork does not.
    write_skill(&w.upstream, "gh", "Upstream v2.");
    commit(&w.upstream, "two");
    sync_and_apply(&w);
    assert!(
        fs::read_to_string(skill_file(&w))
            .unwrap()
            .contains("My fork.")
    );
    assert!(audit(&w.env, &w.scope).unwrap().drift.is_empty());

    // The updates projection knows it is a fork now, not an update.
    let rows = vstack_core::package::updates::updates(&w.env, &w.scope).unwrap();
    let gh = rows.iter().find(|row| row.name == "gh").unwrap();
    assert!(gh.forked);
    assert!(
        !gh.update_available,
        "a local fork has no remote versions to offer: {gh:?}"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn rename_fork_moves_the_declaration_and_refuses_depended_on_names() {
    let w = world();
    write_skill(&w.upstream, "gh", "Upstream.");
    commit(&w.upstream, "one");
    declare(&w, "[skills.gh]\nsource = \"cat\"\n");
    sync_and_apply(&w);
    fs::write(
        skill_file(&w),
        "---\nname: gh\ndescription: mine\n---\nMine.\n",
    )
    .unwrap();
    let plan = fork::fork(&w.env, &w.scope, ItemKind::Skill, "gh", HarnessId::Claude).unwrap();
    apply::execute(&w.env, &plan, None).unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();

    let plan = fork::rename_fork(&w.env, &w.scope, ItemKind::Skill, "gh", "my-gh").unwrap();
    apply::execute(&w.env, &plan, None).unwrap();
    let text = fs::read_to_string(manifest::manifest_path(&w.env, &w.scope)).unwrap();
    assert!(text.contains("[skills.my-gh]"), "{text}");
    assert!(text.contains("[forks.skill.my-gh]"));
    assert!(!text.contains("[skills.gh]"));
    assert!(
        w.home
            .join("app/.vstack-local/skills/my-gh/SKILL.md")
            .is_file()
    );
}
