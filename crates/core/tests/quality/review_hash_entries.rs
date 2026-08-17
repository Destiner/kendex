//! The review hash for what is not a plain file tree: an entry inside a
//! shared config file, a hook in either file shape, and a link inside an
//! item.

use std::fs;

use vstack_core::engine::observed_rows;
use vstack_core::manifest;

use super::fixture::fixture;
use super::review_hash::{install_skill, row};

/// An entry inside shared harness config, hashed on both sides of the write
/// that creates it. The gate reads the entry it is about to write; the audit
/// digs the same entry back out of the file it landed in. A hash that could
/// not survive that round trip would stale every decision the moment
/// somebody acted on it.
#[test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
fn an_mcp_decision_survives_the_write_that_acts_on_it() {
    let f = fixture();
    fs::create_dir_all(f.source.join("mcp")).unwrap();
    fs::write(
        f.source.join("mcp/leaky.toml"),
        "command = \"node\"\nargs = [\"--eval\", \"$(whoami)\"]\n",
    )
    .unwrap();
    let manifest_path = manifest::manifest_path(&f.env, &f.scope);
    let declared =
        fs::read_to_string(&manifest_path).unwrap() + "\n[mcp-servers.leaky]\nsource = \"cat\"\n";
    fs::write(&manifest_path, declared).unwrap();

    let report = vstack_core::engine::audit(&f.env, &f.scope).unwrap();
    let planned = report
        .safety
        .iter()
        .find(|row| row.name == "leaky")
        .expect("the gate scores the server it would write");
    let planned_hash = planned
        .review_hash
        .clone()
        .expect("the entry a plan would write is always readable");
    vstack_core::apply::execute(&f.env, &report.plan, None).unwrap();

    assert_eq!(
        row(&f.env, &f.scope, "leaky").review_hash.as_deref(),
        Some(planned_hash.as_str()),
        "the entry the gate read and the entry the audit found are one entry"
    );
}

/// A hook lives as one registration inside a shared settings file, and the
/// rules score that whole file under the hook's name — so the hash is the
/// file's bytes, whichever shape it takes: handlers nested under a matcher
/// group, or Copilot's entries carrying their action inline. A change
/// anywhere in the file is a change to what was reviewed.
#[test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
fn a_hook_registration_hashes_in_both_file_shapes() {
    let f = fixture();
    let claude = f.project.join(".claude/settings.json");
    fs::write(
        &claude,
        r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"bash /x/guard.sh","timeout":10}]}]}}"#,
    )
    .unwrap();
    let copilot = f.project.join(".github/hooks/guard.json");
    fs::create_dir_all(copilot.parent().unwrap()).unwrap();
    fs::write(
        &copilot,
        r#"{"version":1,"hooks":{"preToolUse":[{"type":"command","bash":"bash /x/guard.sh","matcher":"shell","timeoutSec":10}]}}"#,
    )
    .unwrap();

    let rows = observed_rows(&f.env, &f.scope).unwrap();
    let hook = |harness: vstack_core::model::HarnessId| {
        rows.iter()
            .find(|row| row.kind == vstack_core::model::ItemKind::Hook && row.harness == harness)
            .unwrap_or_else(|| panic!("a {} hook is observed", harness.name()))
            .review_hash
            .clone()
            .expect("a readable registration has a hash")
    };
    let nested = hook(vstack_core::model::HarnessId::Claude);
    let inline = hook(vstack_core::model::HarnessId::Copilot);

    fs::write(
        &claude,
        r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"bash /x/guard.sh","timeout":30}]}]}}"#,
    )
    .unwrap();
    fs::write(
        &copilot,
        r#"{"version":1,"hooks":{"preToolUse":[{"type":"command","bash":"bash /x/guard.sh","matcher":"shell","timeoutSec":30}]}}"#,
    )
    .unwrap();
    let rows = observed_rows(&f.env, &f.scope).unwrap();
    let hook_after = |harness: vstack_core::model::HarnessId| {
        rows.iter()
            .find(|row| row.kind == vstack_core::model::ItemKind::Hook && row.harness == harness)
            .unwrap()
            .review_hash
            .clone()
            .unwrap()
    };
    assert_ne!(nested, hook_after(vstack_core::model::HarnessId::Claude));
    assert_ne!(inline, hook_after(vstack_core::model::HarnessId::Copilot));

    // A key that is not the hook's own entry, in the same file the rules
    // read, is still content nobody reviewed.
    let before = hook_after(vstack_core::model::HarnessId::Claude);
    fs::write(
        &claude,
        r#"{"env":{"SETUP":"chmod 777 /etc/shadow"},"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"bash /x/guard.sh","timeout":30}]}]}}"#,
    )
    .unwrap();
    let rows = observed_rows(&f.env, &f.scope).unwrap();
    let after = rows
        .iter()
        .find(|row| {
            row.kind == vstack_core::model::ItemKind::Hook
                && row.harness == vstack_core::model::HarnessId::Claude
        })
        .unwrap()
        .review_hash
        .clone()
        .unwrap();
    assert_ne!(before, after);
}

/// A link inside an item is hashed as a link — where it points — and never
/// read through: what is past it is somebody else's files, and reading them
/// on every audit would be an unbounded read of wherever the link leads.
/// Repointing the link is a change to the item; editing its target is not.
#[test]
#[allow(clippy::unwrap_used)]
fn a_link_inside_an_item_is_hashed_by_where_it_points() {
    let f = fixture();
    let dir = install_skill(&f, "payload");
    let outside = f.env.home.join("elsewhere");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "one").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("data")).unwrap();
    let before = row(&f.env, &f.scope, "payload").review_hash.unwrap();

    fs::write(outside.join("secret.txt"), "two").unwrap();
    assert_eq!(
        row(&f.env, &f.scope, "payload").review_hash.unwrap(),
        before,
        "bytes past a link are not this item's"
    );

    fs::remove_file(dir.join("data")).unwrap();
    std::os::unix::fs::symlink(f.env.home.join("elsewhere2"), dir.join("data")).unwrap();
    assert_ne!(
        row(&f.env, &f.scope, "payload").review_hash.unwrap(),
        before,
        "where the link points is"
    );
}
