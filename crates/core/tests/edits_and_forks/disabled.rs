//! Edit protection across the enable/disable boundary.

use std::fs;

use super::*;

#[test]
#[allow(clippy::unwrap_used)]
fn an_edit_made_while_disabled_survives_being_re_enabled() {
    let w = world();
    write_skill(&w.upstream, "gh", "Upstream.");
    commit(&w.upstream, "one");
    // Agents render to a File artifact with a `.disabled` sibling — the
    // path that would otherwise be missed.
    let dir = w.upstream.join("agents");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("rev.md"),
        "---\nname: rev\ndescription: reviewer\n---\nReview carefully.\n",
    )
    .unwrap();
    commit(&w.upstream, "agent");
    declare(&w, "[agents.rev]\nsource = \"cat\"\n");
    sync_and_apply(&w);

    // Turn it off, then edit the disabled file, then turn it back on.
    let toggled = manifest::manifest_path(&w.env, &w.scope);
    fs::write(
        &toggled,
        format!(
            "schema = 3\n\n[sources.cat]\nrepo = \"{REPO}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[agents.rev]\nsource = \"cat\"\nenabled = false\n"
        ),
    )
    .unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    apply::execute(&w.env, &report.plan, None).unwrap();
    let disabled = w.home.join("app/.claude/agents/rev.md.disabled");
    assert!(disabled.is_file(), "disabled agent keeps its bytes");
    fs::write(&disabled, "my edited disabled agent").unwrap();

    fs::write(
        &toggled,
        format!(
            "schema = 3\n\n[sources.cat]\nrepo = \"{REPO}\"\n\n[install]\nharnesses = [\"claude\"]\nmethod = \"symlink\"\n\n[agents.rev]\nsource = \"cat\"\n"
        ),
    )
    .unwrap();
    let report = audit(&w.env, &w.scope).unwrap();
    let row = report.drift.iter().find(|row| row.name == "rev").unwrap();
    assert_eq!(
        row.cause,
        Some(DriftCause::LocalEdit),
        "an edit made while off is still an edit: {row:?}"
    );
    apply::execute(&w.env, &report.plan, None).unwrap();
    let enabled = w.home.join("app/.claude/agents/rev.md");
    let content = fs::read_to_string(&enabled)
        .or_else(|_| fs::read_to_string(&disabled))
        .unwrap();
    assert_eq!(content, "my edited disabled agent");
}
