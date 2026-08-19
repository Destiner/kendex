//! Adopting one repository's cached artifacts under another key — the
//! moved default reached by its new spelling still owns what the old
//! spelling downloaded.

use std::fs;

use crate::env::Env;
use crate::error::{CoreError, Result};

use super::{COMMITS, lock_repo, mirror_dir};

pub fn adopt_cache(env: &Env, from: &str, to: &str) -> Result<()> {
    let commits = env.source_cache_dir().join(COMMITS);
    let pairs = [
        (mirror_dir(env, from), mirror_dir(env, to)),
        (commits.join(from), commits.join(to)),
        (
            crate::drift::stamps::stamp_path(env, from),
            crate::drift::stamps::stamp_path(env, to),
        ),
    ];
    if pairs
        .iter()
        .all(|(old, _)| fs::symlink_metadata(old).is_err())
    {
        return Ok(());
    }
    let _guard = match lock_repo(env, to) {
        Ok(guard) => guard,
        // Someone else holds this key's cache — likely mid-fetch or
        // mid-adoption. The next read retries; failing this one would turn
        // a transient lock into a hard error.
        Err(CoreError::CacheBusy { .. }) => return Ok(()),
        Err(error) => return Err(error),
    };
    for (old, new) in pairs {
        if fs::symlink_metadata(&old).is_ok() && fs::symlink_metadata(&new).is_err() {
            if let Some(parent) = new.parent() {
                fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
            }
            fs::rename(&old, &new).map_err(|e| CoreError::io(&old, e))?;
        }
    }
    Ok(())
}
