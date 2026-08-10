use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::fs::read_if_exists;

/// Best-effort git-origin lookup for observed content, memoized per
/// directory so one scan never reads the same `.git/config` twice.
#[derive(Default)]
pub struct OriginCache {
    by_dir: HashMap<PathBuf, Option<String>>,
}

impl OriginCache {
    /// Origin URL of the repository holding `path`'s real (symlink-resolved)
    /// location, if any.
    pub fn origin_of(&mut self, path: &Path) -> Option<String> {
        let real = path.canonicalize().ok()?;
        let start = if real.is_dir() {
            real
        } else {
            real.parent()?.to_path_buf()
        };
        self.lookup(&start)
    }

    fn lookup(&mut self, start: &Path) -> Option<String> {
        let mut visited = Vec::new();
        let mut current = Some(start.to_path_buf());
        let mut found = None;
        while let Some(dir) = current {
            if let Some(cached) = self.by_dir.get(&dir) {
                found = cached.clone();
                break;
            }
            let config = dir.join(".git/config");
            if config.is_file() {
                found = read_if_exists(&config)
                    .ok()
                    .flatten()
                    .and_then(|text| origin_url(&text));
                visited.push(dir);
                break;
            }
            current = dir.parent().map(Path::to_path_buf);
            visited.push(dir);
        }
        for dir in visited {
            self.by_dir.insert(dir, found.clone());
        }
        found
    }
}

/// `url` from the `[remote "origin"]` section of a git config.
fn origin_url(config: &str) -> Option<String> {
    let mut in_origin = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_origin = line == "[remote \"origin\"]";
            continue;
        }
        if in_origin && let Some(url) = line.strip_prefix("url") {
            let url = url.trim_start().strip_prefix('=')?.trim();
            if !url.is_empty() {
                return Some(url.to_owned());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[cfg(unix)]
    #[test]
    fn resolves_origin_through_symlinks_and_memoizes() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::write(
            repo.join(".git/config"),
            "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = git@github.com:o/r.git\n",
        )
        .unwrap();
        fs::create_dir_all(repo.join("skills/x")).unwrap();
        fs::write(repo.join("skills/x/SKILL.md"), "").unwrap();

        let link = tmp.path().join("installed");
        std::os::unix::fs::symlink(repo.join("skills/x"), &link).unwrap();

        let mut cache = OriginCache::default();
        assert_eq!(
            cache.origin_of(&link).as_deref(),
            Some("git@github.com:o/r.git")
        );
        assert_eq!(
            cache.origin_of(&repo.join("skills/x/SKILL.md")).as_deref(),
            Some("git@github.com:o/r.git")
        );
    }

    #[test]
    fn no_repo_means_no_origin() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("loose.md");
        fs::write(&file, "").unwrap();
        let mut cache = OriginCache::default();
        assert_eq!(cache.origin_of(&file), None);
    }
}
