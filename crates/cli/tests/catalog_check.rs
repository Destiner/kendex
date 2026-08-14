//! `vstack check --catalog` as a CI gate: it must fail on content that
//! would not install, and it must pass on what `vstack init` writes.
//! HarnessKit's equivalent always exited 0, which made it unusable for
//! exactly this.
#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

#[allow(clippy::expect_used)]
fn vstack(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vstack"))
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .output()
        .expect("vstack binary runs")
}

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bad-catalog")
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_seeded_bad_catalog_fails_the_check() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let catalog = fixture();
    let output = vstack(
        home,
        home,
        &["check", "--catalog", catalog.to_str().unwrap()],
    );

    assert!(!output.status.success(), "a broken catalog must not pass");
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    // Both passes have to have run: structure and safety.
    assert!(said.contains("[error] safety:"), "{said}");
    assert!(said.contains("rce"), "{said}");
    assert!(said.contains("prompt-injection"), "{said}");
    assert!(said.contains("credential-theft"), "{said}");
    // The capitalised agent name is a loader problem, not a safety one.
    assert!(said.contains("lowercase letters"), "{said}");
    // Every finding travels with its fix.
    assert!(said.contains("    fix: "), "{said}");
}

/// The scaffolding vstack writes must survive vstack's own gate. A starting
/// point that fails the check on its first run teaches people to ignore it.
#[test]
#[allow(clippy::unwrap_used)]
fn what_init_scaffolds_passes_the_check() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let catalog = home.join("catalog");
    std::fs::create_dir_all(&catalog).unwrap();

    for (name, kind) in [
        ("reviewer", "agent"),
        ("release-notes", "skill"),
        ("guard-bash", "hook"),
    ] {
        let output = vstack(home, &catalog, &["init", name, "--kind", kind]);
        assert!(
            output.status.success(),
            "init {kind} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = vstack(
        home,
        home,
        &["check", "--catalog", catalog.to_str().unwrap()],
    );
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{said}");
    assert!(said.contains("3 item(s)"), "{said}");
    assert!(said.contains("0 breakage"), "{said}");
    assert!(said.contains("0 held back"), "{said}");
}
