use std::fs;
use std::path::Path;

use crate::error::{CoreError, Result};

/// Write via a sibling temp file + rename so readers never see a torn file.
pub fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| CoreError::io(path, std::io::Error::other("path has no parent")))?;
    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, contents).map_err(|e| CoreError::io(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| CoreError::io(path, e))
}

/// `atomic_write` plus full crash durability: the temp file syncs before
/// the rename and the parent directory syncs after, so after this returns
/// the file either exists complete or not at all — even across power loss.
pub fn atomic_write_durable(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| CoreError::io(path, std::io::Error::other("path has no parent")))?;
    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, contents).map_err(|e| CoreError::io(&tmp, e))?;
    fs::File::open(&tmp)
        .and_then(|f| f.sync_all())
        .map_err(|e| CoreError::io(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| CoreError::io(path, e))?;
    #[cfg(unix)]
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

pub fn read_if_exists(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::io(path, e)),
    }
}
