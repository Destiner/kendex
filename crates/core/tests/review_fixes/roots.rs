//! Where a tool keeps its files, and what the audit sees when that is not
//! the default place.

use kendex_core::engine::{DriftState, audit};
use kendex_core::model::{HarnessId, Scope};

use super::{put, world};

/// A tool pointed at a folder of its own still has its hand-made items
/// found. Auditing the default folder instead reports an empty machine and
/// silently hides everything the user put where they said to put it.
#[test]
fn a_relocated_tool_folder_is_still_scanned_for_unmanaged_items() {
    let w = world();
    let elsewhere = w.home.join("elsewhere/claude");
    let mut settings = kendex_core::settings::load(&w.env).unwrap();
    settings
        .harness_roots
        .insert("claude".into(), elsewhere.clone());
    kendex_core::settings::save(&w.env, &settings).unwrap();

    put(
        &elsewhere.join("skills/handmade/SKILL.md"),
        "---\nname: handmade\n---\nby hand\n",
    );

    let report = audit(&w.env, &Scope::Global).unwrap();

    assert!(
        report.drift.iter().any(|row| {
            row.name == "handmade"
                && row.harness == HarnessId::Claude
                && row.state == DriftState::Unmanaged
        }),
        "relocated folder was not scanned: {:?}",
        report.drift
    );
}
