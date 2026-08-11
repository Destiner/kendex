use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::fs::atomic_write;

/// Pre-images of everything an apply is about to touch. Restore is
/// idempotent, so a crash mid-rollback recovers by rolling back again.
#[derive(Debug, Serialize, Deserialize)]
pub struct Journal {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    pub path: PathBuf,
    pub state: PreState,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum PreState {
    Absent,
    /// Bytes stored under `store/<index>` in the journal dir.
    File {
        store: String,
    },
    Symlink {
        target: PathBuf,
    },
    /// Tree copied under `store/<index>/` in the journal dir.
    Dir {
        store: String,
    },
}

pub fn journal_dir_for(base: &Path, scope_key: &str) -> PathBuf {
    base.join(scope_key)
}

/// Record pre-images for every path, then durably write the journal meta.
/// Only after this returns may the apply mutate anything.
pub fn write(dir: &Path, paths: &[PathBuf]) -> Result<()> {
    let store = dir.join("store");
    fs::create_dir_all(&store).map_err(|e| CoreError::io(&store, e))?;
    let mut entries = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let state = if path.is_symlink() {
            PreState::Symlink {
                target: fs::read_link(path).map_err(|e| CoreError::io(path, e))?,
            }
        } else if path.is_dir() {
            let slot = store.join(index.to_string());
            copy_tree(path, &slot)?;
            PreState::Dir {
                store: index.to_string(),
            }
        } else if path.is_file() {
            let slot = store.join(index.to_string());
            fs::copy(path, &slot).map_err(|e| CoreError::io(path, e))?;
            PreState::File {
                store: index.to_string(),
            }
        } else {
            PreState::Absent
        };
        entries.push(Entry {
            path: path.clone(),
            state,
        });
    }
    let journal = Journal { entries };
    let meta = serde_json::to_string_pretty(&journal).map_err(|e| CoreError::JsonParse {
        path: dir.join("meta.json"),
        message: e.to_string(),
    })?;
    atomic_write(&dir.join("meta.json"), &meta)?;
    let file = fs::File::open(dir.join("meta.json")).map_err(|e| CoreError::io(dir, e))?;
    file.sync_all().map_err(|e| CoreError::io(dir, e))?;
    Ok(())
}

/// Restore every pre-image. Used both for in-process rollback and for
/// crash recovery on the next run.
pub fn rollback(dir: &Path) -> Result<()> {
    let meta_path = dir.join("meta.json");
    let Some(text) = crate::fs::read_if_exists(&meta_path)? else {
        // Mutation never started (no meta written): nothing to restore.
        return clear(dir);
    };
    let journal: Journal = serde_json::from_str(&text).map_err(|e| CoreError::JsonParse {
        path: meta_path,
        message: e.to_string(),
    })?;
    let store = dir.join("store");
    for entry in &journal.entries {
        remove_any(&entry.path)?;
        match &entry.state {
            PreState::Absent => {}
            PreState::File { store: slot } => {
                if let Some(parent) = entry.path.parent() {
                    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
                }
                fs::copy(store.join(slot), &entry.path)
                    .map_err(|e| CoreError::io(&entry.path, e))?;
            }
            PreState::Symlink { target } => {
                if let Some(parent) = entry.path.parent() {
                    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
                }
                make_symlink(target, &entry.path)?;
            }
            PreState::Dir { store: slot } => {
                copy_tree(&store.join(slot), &entry.path)?;
            }
        }
    }
    clear(dir)
}

pub fn clear(dir: &Path) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    }
    Ok(())
}

pub fn pending(dir: &Path) -> bool {
    dir.join("meta.json").is_file()
}

pub fn remove_any(path: &Path) -> Result<()> {
    if path.is_symlink() || path.is_file() {
        fs::remove_file(path).map_err(|e| CoreError::io(path, e))?;
    } else if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| CoreError::io(path, e))?;
    }
    Ok(())
}

pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).map_err(|e| CoreError::io(to, e))?;
    for entry in fs::read_dir(from)
        .map_err(|e| CoreError::io(from, e))?
        .flatten()
    {
        let source = entry.path();
        let Some(name) = source.file_name() else {
            continue;
        };
        let dest = to.join(name);
        if source.is_symlink() {
            let target = fs::read_link(&source).map_err(|e| CoreError::io(&source, e))?;
            make_symlink(&target, &dest)?;
        } else if source.is_dir() {
            copy_tree(&source, &dest)?;
        } else {
            fs::copy(&source, &dest).map_err(|e| CoreError::io(&source, e))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
pub fn make_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link).map_err(|e| CoreError::io(link, e))
}

#[cfg(windows)]
pub fn make_symlink(target: &Path, link: &Path) -> Result<()> {
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link).map_err(|e| CoreError::io(link, e))
    } else {
        std::os::windows::fs::symlink_file(target, link).map_err(|e| CoreError::io(link, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_restores_files_dirs_symlinks_and_absence() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("work");
        fs::create_dir_all(work.join("tree/sub")).unwrap();
        fs::write(work.join("file.md"), "original").unwrap();
        fs::write(work.join("tree/sub/x"), "x").unwrap();
        make_symlink(Path::new("/nowhere"), &work.join("link")).unwrap();
        let absent = work.join("was-absent");

        let journal_dir = tmp.path().join("journal/global");
        write(
            &journal_dir,
            &[
                work.join("file.md"),
                work.join("tree"),
                work.join("link"),
                absent.clone(),
            ],
        )
        .unwrap();

        fs::write(work.join("file.md"), "clobbered").unwrap();
        fs::remove_dir_all(work.join("tree")).unwrap();
        fs::remove_file(work.join("link")).unwrap();
        fs::write(&absent, "should vanish").unwrap();

        rollback(&journal_dir).unwrap();

        assert_eq!(
            fs::read_to_string(work.join("file.md")).unwrap(),
            "original"
        );
        assert_eq!(fs::read_to_string(work.join("tree/sub/x")).unwrap(), "x");
        assert_eq!(
            fs::read_link(work.join("link")).unwrap(),
            Path::new("/nowhere")
        );
        assert!(!absent.exists());
        assert!(!pending(&journal_dir));
    }
}
