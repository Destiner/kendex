//! Blocking at apply: what the plan says about content it would write, and
//! what never reaches the disk.

use vstack_core::apply;
use vstack_core::engine::DriftState;
use vstack_core::quality::Verdict;

use super::fixture::{fixture, installed, plan};

/// The plan carries both scores for every item it would write, and the
/// blocked one never reaches the op list.
#[test]
#[allow(clippy::unwrap_used)]
fn a_critical_finding_holds_an_item_back_and_installs_the_rest() {
    let f = fixture();
    let report = plan(&f, &[]);

    let hostile = report
        .safety
        .iter()
        .find(|row| row.name == "hostile")
        .unwrap();
    assert_eq!(hostile.verdict, Verdict::Block);
    assert_eq!(hostile.safety.score, 75);
    assert!(hostile.blocked());
    assert!(hostile.quality.is_some(), "a skill has authored prose");

    let clean = report
        .safety
        .iter()
        .find(|row| row.name == "clean")
        .unwrap();
    assert_eq!(clean.verdict, Verdict::Clean);
    assert_eq!(clean.safety.score, 100);

    // The conflict row says why, in the same machinery a refused rendering
    // already uses.
    let row = report
        .drift
        .iter()
        .find(|row| row.name == "hostile")
        .unwrap();
    assert_eq!(row.state, DriftState::Conflict);
    assert!(row.detail.contains("held back by the safety check"));

    apply::execute(&f.env, &report.plan, None).unwrap();
    assert!(installed(&f, "clean"));
    assert!(!installed(&f, "hostile"));
}
