use std::path::Path;

use specta_typescript::Typescript;

fn exporter() -> Typescript {
    Typescript::default().header("// @ts-nocheck")
}

fn committed_path() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../ui/src/bindings.ts"
    ))
}

/// `cargo test` fails whenever the committed bindings drift from the command
/// surface. Regenerate with:
/// `cargo test -p vstack-app -- --ignored regenerate_bindings`
#[test]
fn committed_bindings_are_current() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let fresh_path = tmp.path().join("bindings.ts");
    vstack_app::specta_builder()
        .export(exporter(), &fresh_path)
        .expect("bindings export");
    let fresh = std::fs::read_to_string(&fresh_path).expect("fresh bindings readable");
    let committed = std::fs::read_to_string(committed_path()).unwrap_or_default();
    assert_eq!(
        committed, fresh,
        "ui/src/bindings.ts is stale — run: cargo test -p vstack-app -- --ignored regenerate_bindings"
    );
}

#[test]
#[ignore = "writes ui/src/bindings.ts in place"]
fn regenerate_bindings() {
    vstack_app::specta_builder()
        .export(exporter(), committed_path())
        .expect("bindings export");
}
