#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

// Integration-test helpers sit outside #[test] fns, so clippy's
// allow-unwrap-in-tests does not reach them.
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

#[allow(clippy::unwrap_used)]
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A project that declares one pi extension from a local catalog and already
/// has an older copy of it installed under `.pi/packages/`.
#[allow(clippy::unwrap_used)]
fn fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("dev/app");
    write(
        &project.join("vstack.toml"),
        "schema = 1\n\n[sources.cat]\npath = \"catalog\"\n\n[pi-extensions.pi-widgets]\nsource = \"cat\"\n",
    );
    let package = "{\n  \"name\": \"pi-widgets\",\n  \"version\": \"2.0.0\",\n  \"pi\": { \"extensions\": [\"index.js\"] }\n}\n";
    write(
        &project.join("catalog/pi-extensions/pi-widgets/package.json"),
        package,
    );
    write(
        &project.join("catalog/pi-extensions/pi-widgets/index.js"),
        "export const version = 2;\n",
    );

    write(
        &project.join(".pi/packages/pi-widgets/package.json"),
        package,
    );
    write(
        &project.join(".pi/packages/pi-widgets/index.js"),
        "export const version = 1;\n",
    );
    write(
        &project.join(".pi/settings.json"),
        "{\"packages\": [\"./packages/pi-widgets\"]}\n",
    );
    tmp
}

#[test]
fn check_reports_stale_packages_without_touching_them() {
    let tmp = fixture();
    let project = tmp.path().join("dev/app");
    let installed = project.join(".pi/packages/pi-widgets/index.js");

    let output = vstack(tmp.path(), &project, &["update-pi", "--check"]);

    assert!(output.status.success());
    let plan = String::from_utf8_lossy(&output.stdout);
    assert!(plan.contains("pi-widgets"), "{plan}");
    assert!(plan.contains("stale"), "{plan}");
    let summary = String::from_utf8_lossy(&output.stderr);
    assert!(summary.contains("1 package(s) can be updated"), "{summary}");
    assert_eq!(
        fs::read_to_string(&installed).unwrap(),
        "export const version = 1;\n"
    );
}

#[test]
fn update_reinstalls_from_the_declared_source() {
    let tmp = fixture();
    let project = tmp.path().join("dev/app");
    let installed = project.join(".pi/packages/pi-widgets/index.js");

    let output = vstack(tmp.path(), &project, &["update-pi"]);

    assert!(output.status.success());
    let progress = String::from_utf8_lossy(&output.stdout);
    assert!(
        progress.contains("updated pi-widgets -> 2.0.0"),
        "{progress}"
    );
    assert_eq!(
        fs::read_to_string(&installed).unwrap(),
        "export const version = 2;\n"
    );

    // A second run has nothing left to do.
    let output = vstack(tmp.path(), &project, &["update-pi"]);
    assert!(output.status.success());
    let summary = String::from_utf8_lossy(&output.stderr);
    assert!(summary.contains("all pi packages up to date"), "{summary}");
}

#[test]
fn a_package_no_source_declares_is_reported_not_updated() {
    let tmp = fixture();
    let project = tmp.path().join("dev/app");
    fs::remove_dir_all(project.join("catalog/pi-extensions/pi-widgets")).unwrap();

    let output = vstack(tmp.path(), &project, &["update-pi"]);

    assert!(output.status.success());
    let plan = String::from_utf8_lossy(&output.stdout);
    assert!(plan.contains("no declared source"), "{plan}");
    let notes = String::from_utf8_lossy(&output.stderr);
    assert!(notes.contains("no longer ships pi-extensions"), "{notes}");
}
