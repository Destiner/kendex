//! A tool vstack can only read is found and reported, and never seeded as a
//! place to install into — a target whose every write is a no-op would show
//! up in the manifest as intent nothing acts on.
#![cfg(unix)]

use std::fs;

use vstack_core::engine::ops;
use vstack_core::env::{Env, FakeOs};
use vstack_core::model::{HarnessId, Scope};
use vstack_core::scan;
use vstack_core::settings::AppSettings;

#[test]
#[allow(clippy::unwrap_used)]
fn a_fresh_manifest_targets_only_the_tools_vstack_writes_to() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let env = Env::fake(home, FakeOs::Linux);
    for root in [".claude", ".gemini", ".copilot"] {
        fs::create_dir_all(home.join(root)).unwrap();
    }

    let detected: Vec<_> = scan::scan(&env, &AppSettings::default())
        .harnesses
        .iter()
        .map(|h| h.harness)
        .collect();
    assert_eq!(
        detected,
        [HarnessId::Claude, HarnessId::Gemini, HarnessId::Copilot]
    );

    // Copilot is the read-only one: it is found and reported, and left out
    // of the manifest because every write to it would be a no-op.
    let manifest = ops::manifest_for_mutation(&env, &Scope::Global).unwrap();
    assert_eq!(
        manifest.install.harnesses,
        [HarnessId::Claude, HarnessId::Gemini]
    );
}
