use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

/// Write via a sibling temp file + rename so readers never see a torn file.
/// A symlink at the path is followed: the file it points at is replaced and
/// the link stays — renaming over the link itself would swap a user's
/// dotfiles link for a detached copy.
pub fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    write_then_rename(path, contents, false)
}

/// `atomic_write` plus full crash durability: the temp file syncs before
/// the rename and the parent directory syncs after, so after this returns
/// the file either exists complete or not at all — even across power loss.
pub fn atomic_write_durable(path: &Path, contents: &str) -> Result<()> {
    write_then_rename(path, contents, true)
}

fn write_then_rename(path: &Path, contents: &str, durable: bool) -> Result<()> {
    let path = follow_link(path);
    let parent = path
        .parent()
        .ok_or_else(|| CoreError::io(&path, std::io::Error::other("path has no parent")))?;
    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, contents).map_err(|e| CoreError::io(&tmp, e))?;
    if durable {
        fs::File::open(&tmp)
            .and_then(|f| f.sync_all())
            .map_err(|e| CoreError::io(&tmp, e))?;
    }
    fs::rename(&tmp, &path).map_err(|e| CoreError::io(&path, e))?;
    #[cfg(unix)]
    if durable && let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// The file a symlink chain ends at; the path itself when it is not a link
/// or the link is broken (nothing to preserve, the rename replaces it).
fn follow_link(path: &Path) -> PathBuf {
    match path.is_symlink() {
        true => fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
        false => path.to_path_buf(),
    }
}

pub fn read_if_exists(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::io(path, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_symlinked_file_is_rewritten_through_the_link() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("dotfiles/kendex.toml");
        fs::create_dir_all(real.parent().unwrap()).unwrap();
        fs::write(&real, "old").unwrap();
        let link = tmp.path().join("kendex.toml");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        atomic_write(&link, "new").unwrap();
        atomic_write_durable(&link, "newer").unwrap();

        assert!(link.is_symlink());
        assert_eq!(fs::read_to_string(&real).unwrap(), "newer");
    }
}
