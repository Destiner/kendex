//! What happens to a dismissal when the item it is about moves: rebound to
//! another source, renamed, or taken away with whatever needed it.

use std::fs;

use vstack_core::apply;
use vstack_core::engine::audit;
use vstack_core::engine::decisions::DecisionState;
use vstack_core::engine::ops;
use vstack_core::manifest;
use vstack_core::quality::reviews::DismissReason;

use super::decisions::{MILD_KEY, dismiss_first, row, with_mild};
use super::fixture::{fixture, installed, manifest_of, skill};

/// Trusting a source is trusting *that* source. The same bytes served from
/// somewhere else are not what was trusted, and the record says so.
#[test]
#[allow(clippy::unwrap_used)]
fn a_trusted_source_dismissal_binds_to_the_source() {
    let f = with_mild();
    let trusted = row(&f, "mild").provenance.clone().unwrap();
    let row_hash = row(&f, "mild").review_hash.clone();
    dismiss_first(&f, "mild", DismissReason::TrustedSource);
    let recorded = manifest_of(&f);
    let dismissal = recorded.safety_reviews[MILD_KEY]
        .dismissed
        .values()
        .next()
        .unwrap();
    assert_eq!(dismissal.source.as_deref(), Some(trusted.as_str()));
    assert!(matches!(
        row(&f, "mild").decisions[0].state,
        DecisionState::Dismissed { .. }
    ));

    // A fork keeps the name and the bytes and rebinds the item to the local
    // source — exactly the move a trusted-source decision must not survive.
    let forked = vstack_core::engine::fork::fork(
        &f.env,
        &f.scope,
        vstack_core::model::ItemKind::Skill,
        "mild",
        vstack_core::model::HarnessId::Claude,
    )
    .unwrap();
    apply::execute(&f.env, &forked, None).unwrap();
    let report = audit(&f.env, &f.scope).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();

    let after = row(&f, "mild");
    assert_eq!(after.provenance.as_deref(), Some("local"));
    assert_eq!(after.review_hash, row_hash, "the bytes did not move");
    match &after.decisions[0].state {
        DecisionState::Open { earlier: Some(why) } => assert!(why.contains("it trusted"), "{why}"),
        other => panic!("expected an open finding that says why, got {other:?}"),
    }
}

/// A rename moves the item's decisions with it. What must not happen is a
/// record left under the old name, waiting for something else to take it.
#[test]
#[allow(clippy::unwrap_used)]
fn renaming_an_item_carries_its_decision_and_leaves_nothing_behind() {
    use vstack_core::model::{HarnessId, ItemKind};
    let f = with_mild();
    let forked = vstack_core::engine::fork::fork(
        &f.env,
        &f.scope,
        ItemKind::Skill,
        "mild",
        HarnessId::Claude,
    )
    .unwrap();
    apply::execute(&f.env, &forked, None).unwrap();
    apply::execute(&f.env, &audit(&f.env, &f.scope).unwrap().plan, None).unwrap();
    dismiss_first(&f, "mild", DismissReason::Intended);

    let renamed =
        vstack_core::engine::fork::rename_fork(&f.env, &f.scope, ItemKind::Skill, "mild", "gentle")
            .unwrap();
    apply::execute(&f.env, &renamed, None).unwrap();

    let recorded = manifest_of(&f);
    assert!(
        !recorded.safety_reviews.contains_key(MILD_KEY),
        "{recorded:?}"
    );
    let moved = recorded.safety_reviews.get("skill:gentle:claude").unwrap();
    assert_eq!(moved.dismissed.len(), 1);
}

/// What a removal takes away is known only once the plan is made: a swept
/// dependency, a bundle's members. Their decisions go with them, the same as
/// a name asked for directly — a record left behind would speak for a
/// reinstall nobody has looked at.
#[test]
#[allow(clippy::unwrap_used)]
fn removing_what_needed_an_item_reaps_the_items_decisions_too() {
    let f = fixture();
    skill(
        &f.source,
        "mild",
        "Run chmod 777 build.sh before anything else.\n",
    );
    let parent = f.source.join("skills/parent");
    fs::create_dir_all(&parent).unwrap();
    fs::write(
        parent.join("SKILL.md"),
        "---\nname: parent\ndescription: Use this to set things up.\ndependencies:\n  required: [mild]\n---\n\nUse mild.\n",
    )
    .unwrap();
    let path = manifest::manifest_path(&f.env, &f.scope);
    let declared = fs::read_to_string(&path).unwrap() + "\n[skills.parent]\nsource = \"cat\"\n";
    fs::write(&path, declared).unwrap();
    apply::execute(&f.env, &audit(&f.env, &f.scope).unwrap().plan, None).unwrap();
    assert!(
        installed(&f, "mild"),
        "the dependency came in with its parent"
    );
    dismiss_first(&f, "mild", DismissReason::WrongCall);
    assert_eq!(manifest_of(&f).safety_reviews.len(), 1);

    let report = ops::remove(&f.env, &f.scope, &["parent".to_owned()], None, true).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(!installed(&f, "mild"), "the sweep took the dependency");
    assert!(
        manifest_of(&f).safety_reviews.is_empty(),
        "{:?}",
        manifest_of(&f).safety_reviews
    );
}
