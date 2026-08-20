//! The one-shot move of the global dirs off the old `vstack2` product name
//! onto `kendex`, run on first launch under the apply scope-lock.

use std::fs;
use std::path::Path;

use crate::env::Env;
use crate::error::{CoreError, Result};

/// What the one-shot dir move accomplished: whether anything moved, and
/// one line per old dir that still holds items the move could not take —
/// the caller shows those lines instead of the leftovers sitting silently
/// until every later launch re-walks them.
#[derive(Debug, Default)]
pub struct DirMove {
    pub moved: bool,
    pub leftovers: Vec<String>,
}

/// One-shot move of the global dirs off the old product name, on first
/// launch: when a `vstack2` dir exists, its contents move under `kendex`
/// — never overwriting anything already there. Runs under the same
/// scope-lock discipline as an apply: a concurrent kendex process (app
/// or CLI) gets a clear busy error, never an interleaved move.
pub fn migrate_global_dirs(env: &Env) -> Result<DirMove> {
    let mut outcome = DirMove::default();
    if env
        .app_dir_pairs()
        .iter()
        .all(|(old, _)| fs::symlink_metadata(old).is_err())
    {
        return Ok(outcome);
    }
    let _guard = crate::apply::lock_key(env, "rename-from-vstack2")?;
    for (old, new) in env.app_dir_pairs() {
        outcome.moved |= merge_move(&old, &new)?;
        let stayed = leftover_count(&old);
        if stayed > 0 {
            outcome.leftovers.push(format!(
                "{stayed} item(s) stayed in {} — {} already has entries with those names; move or delete them by hand",
                old.display(),
                new.display()
            ));
        }
    }
    Ok(outcome)
}

/// Move `old` to `new`; where both are real directories, move children
/// one by one and take the emptied dir away. A name present on both
/// sides stays put on both — the new side is never overwritten. Symlinks
/// move like files and are never followed: recursing through one would
/// drag in a tree that was never under the old dir.
fn merge_move(old: &Path, new: &Path) -> Result<bool> {
    if fs::symlink_metadata(old).is_err() {
        return Ok(false);
    }
    if fs::symlink_metadata(new).is_err() {
        fs::rename(old, new).map_err(|e| CoreError::io(old, e))?;
        return Ok(true);
    }
    if !(is_real_dir(old) && is_real_dir(new)) {
        return Ok(false);
    }
    // Renaming entries out of a directory while its iterator is live is
    // platform-defined, so the listing is taken up front.
    let entries: Vec<_> = fs::read_dir(old)
        .map_err(|e| CoreError::io(old, e))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|e| CoreError::io(old, e))?;
    let mut moved = false;
    for entry in entries {
        moved |= merge_move(&entry.path(), &new.join(entry.file_name()))?;
    }
    let mut leftovers = fs::read_dir(old).map_err(|e| CoreError::io(old, e))?;
    if leftovers.next().is_none() {
        fs::remove_dir(old).map_err(|e| CoreError::io(old, e))?;
    }
    Ok(moved)
}

fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_dir())
}

/// How many items still sit under `path`, symlinks counted as themselves.
/// A count for a report line, so a dir that cannot be enumerated counts
/// as one item rather than failing the move that already happened.
fn leftover_count(path: &Path) -> usize {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.file_type().is_dir() {
        return 1;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 1;
    };
    entries
        .flatten()
        .map(|entry| leftover_count(&entry.path()))
        .sum::<usize>()
        .max(1)
}
