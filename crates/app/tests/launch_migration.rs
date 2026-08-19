//! The global-dir move gates the launch: state must be under the new
//! names before the app reads them, and a failed move stops the launch —
//! the same refusal the CLI makes — instead of opening an app that would
//! write fresh state beside the stranded old files and fork the library.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use kendex_core::env::{Env, FakeOs};

#[test]
#[allow(clippy::unwrap_used)]
fn launch_preparation_moves_the_old_dirs_before_recovery_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::fake(tmp.path(), FakeOs::Linux);
    fs::create_dir_all(tmp.path().join(".config/vstack2")).unwrap();
    fs::write(
        tmp.path().join(".config/vstack2/settings.toml"),
        "projects = []\n",
    )
    .unwrap();

    kendex_app::prepare_launch(&env).unwrap();
    assert_eq!(
        fs::read_to_string(tmp.path().join(".config/kendex/settings.toml")).unwrap(),
        "projects = []\n"
    );
    assert!(!tmp.path().join(".config/vstack2").exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_failed_move_is_a_launch_error_not_a_shrug() {
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::fake(tmp.path(), FakeOs::Linux);
    let old = tmp.path().join(".config/vstack2");
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("settings.toml"), "projects = []\n").unwrap();
    // A same-named new dir forces the per-entry merge, and an unreadable
    // old dir makes that merge fail the way a permissions problem would.
    fs::create_dir_all(tmp.path().join(".config/kendex")).unwrap();
    fs::set_permissions(&old, fs::Permissions::from_mode(0o000)).unwrap();

    let result = kendex_app::prepare_launch(&env);
    fs::set_permissions(&old, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err(), "{result:?}");
}
