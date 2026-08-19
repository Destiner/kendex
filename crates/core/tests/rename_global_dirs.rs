//! Global dirs move off `vstack2` on first launch: contents merge under
//! `kendex` without ever overwriting what the new dirs hold, symlinks
//! move as links, and whatever could not move is said out loud.
#![cfg(unix)]

use std::fs;
use std::path::PathBuf;

use kendex_core::env::{Env, FakeOs};
use kendex_core::rename::migrate_global_dirs;

struct Fixture {
    _tmp: tempfile::TempDir,
    env: Env,
    home: PathBuf,
}

#[allow(clippy::unwrap_used)]
fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    Fixture {
        env,
        home,
        _tmp: tmp,
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn global_dirs_move_off_vstack2_once() {
    let f = fixture();
    fs::create_dir_all(f.home.join(".config/vstack2")).unwrap();
    fs::write(
        f.home.join(".config/vstack2/settings.toml"),
        "projects = []\n",
    )
    .unwrap();
    fs::create_dir_all(f.home.join(".cache/vstack2/sources/mirrors")).unwrap();
    fs::write(f.home.join(".cache/vstack2/sources/mirrors/HEAD"), "ref").unwrap();
    for child in [
        "trash",
        "journal",
        "locks",
        "drift",
        "local-source",
        "rendered",
    ] {
        let dir = f.home.join(".local/share/vstack2").join(child);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("keep"), child).unwrap();
    }

    let outcome = migrate_global_dirs(&f.env).unwrap();
    assert!(outcome.moved);
    assert_eq!(outcome.leftovers, Vec::<String>::new());
    assert_eq!(
        fs::read_to_string(f.home.join(".config/kendex/settings.toml")).unwrap(),
        "projects = []\n"
    );
    assert!(f.home.join(".cache/kendex/sources/mirrors/HEAD").is_file());
    for child in [
        "trash",
        "journal",
        "locks",
        "drift",
        "local-source",
        "rendered",
    ] {
        assert!(
            f.home
                .join(".local/share/kendex")
                .join(child)
                .join("keep")
                .is_file(),
            "{child} did not move"
        );
    }
    assert!(!f.home.join(".config/vstack2").exists());
    assert!(!f.home.join(".cache/vstack2").exists());
    assert!(!f.home.join(".local/share/vstack2").exists());

    // Nothing left to move: the second pass is a no-op.
    assert!(!migrate_global_dirs(&f.env).unwrap().moved);
}

#[test]
#[allow(clippy::unwrap_used)]
fn the_move_never_overwrites_what_the_new_dirs_hold() {
    let f = fixture();
    fs::create_dir_all(f.home.join(".config/vstack2")).unwrap();
    fs::write(f.home.join(".config/vstack2/settings.toml"), "old").unwrap();
    fs::create_dir_all(f.home.join(".config/kendex")).unwrap();
    fs::write(f.home.join(".config/kendex/settings.toml"), "new").unwrap();
    fs::create_dir_all(f.home.join(".local/share/vstack2/journal/scope-a")).unwrap();
    fs::write(
        f.home.join(".local/share/vstack2/journal/scope-a/entry"),
        "old journal",
    )
    .unwrap();
    fs::create_dir_all(f.home.join(".local/share/kendex/journal/scope-a")).unwrap();
    fs::write(
        f.home.join(".local/share/kendex/journal/scope-a/entry"),
        "new journal",
    )
    .unwrap();

    let outcome = migrate_global_dirs(&f.env).unwrap();
    // What could not move is said once per dir, count and place named —
    // not silently re-walked on every later launch.
    assert_eq!(outcome.leftovers.len(), 2, "{:?}", outcome.leftovers);
    assert!(
        outcome
            .leftovers
            .iter()
            .any(|line| line.contains("1 item(s)") && line.contains(".config/vstack2")),
        "{:?}",
        outcome.leftovers
    );
    assert!(
        outcome
            .leftovers
            .iter()
            .any(|line| line.contains(".local/share/vstack2")),
        "{:?}",
        outcome.leftovers
    );
    assert_eq!(
        fs::read_to_string(f.home.join(".config/kendex/settings.toml")).unwrap(),
        "new"
    );
    assert_eq!(
        fs::read_to_string(f.home.join(".config/vstack2/settings.toml")).unwrap(),
        "old"
    );
    assert_eq!(
        fs::read_to_string(f.home.join(".local/share/kendex/journal/scope-a/entry")).unwrap(),
        "new journal"
    );
    assert_eq!(
        fs::read_to_string(f.home.join(".local/share/vstack2/journal/scope-a/entry")).unwrap(),
        "old journal"
    );
}

/// A symlink under the old dir moves as the link itself — the tree it
/// points at is never walked, and a collision leaves it where it was.
#[test]
#[allow(clippy::unwrap_used)]
fn the_move_carries_symlinks_as_links_and_never_follows_them() {
    let f = fixture();
    let elsewhere = f.home.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    fs::write(elsewhere.join("keep"), "outside data").unwrap();
    fs::create_dir_all(f.home.join(".config/vstack2")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, f.home.join(".config/vstack2/linked")).unwrap();
    fs::create_dir_all(f.home.join(".config/kendex/linked")).unwrap();

    let outcome = migrate_global_dirs(&f.env).unwrap();
    // The pointed-at tree is untouched, on both sides of the collision.
    assert_eq!(
        fs::read_to_string(elsewhere.join("keep")).unwrap(),
        "outside data"
    );
    assert_eq!(
        fs::read_dir(f.home.join(".config/kendex/linked"))
            .unwrap()
            .count(),
        0
    );
    assert!(f.home.join(".config/vstack2/linked").is_symlink());
    assert_eq!(outcome.leftovers.len(), 1, "{:?}", outcome.leftovers);
    assert!(outcome.leftovers[0].contains(".config/vstack2"));

    // Without a collision the link itself moves, still pointing where it
    // pointed.
    fs::remove_dir(f.home.join(".config/kendex/linked")).unwrap();
    let outcome = migrate_global_dirs(&f.env).unwrap();
    assert!(outcome.moved);
    assert_eq!(outcome.leftovers, Vec::<String>::new());
    let landed = f.home.join(".config/kendex/linked");
    assert!(landed.is_symlink());
    assert_eq!(fs::read_link(&landed).unwrap(), elsewhere);
    assert!(!f.home.join(".config/vstack2").exists());
}
