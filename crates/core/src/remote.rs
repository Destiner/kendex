use std::path::PathBuf;
use std::process::Command;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::Manifest;

/// `owner/repo` → clone URL. Full URLs pass through untouched.
pub fn clone_url(repo: &str) -> String {
    if repo.contains("://") || repo.starts_with("git@") {
        repo.to_owned()
    } else {
        format!("https://github.com/{repo}.git")
    }
}

pub fn cache_dir(env: &Env, repo: &str) -> PathBuf {
    env.source_cache_dir().join(repo.replace('/', "_"))
}

fn git(args: &[&str], cwd: Option<&std::path::Path>) -> Result<()> {
    let mut command = Command::new("git");
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .map_err(|e| CoreError::io(PathBuf::from("git"), e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CoreError::GitFailed {
            command: format!("git {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

/// Make sure the cache holds a clone; fetch + hard-reset when it already
/// does (v1 flow). Returns the cache path and a warning instead of an
/// error when a refresh of an existing clone fails — the cached version
/// keeps working offline.
pub fn sync(env: &Env, repo: &str, url: &str) -> Result<(PathBuf, Option<String>)> {
    let cache = cache_dir(env, repo);
    if cache.join(".git").is_dir() {
        let refreshed = git(&["fetch", "origin", "--quiet"], Some(&cache))
            .and_then(|()| git(&["reset", "--hard", "origin/HEAD", "--quiet"], Some(&cache)));
        return Ok(match refreshed {
            Ok(()) => (cache, None),
            Err(error) => (
                cache,
                Some(format!("{repo}: using cached version ({error})")),
            ),
        });
    }
    if let Some(parent) = cache.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    git(
        &["clone", "--quiet", url, &cache.display().to_string()],
        None,
    )?;
    Ok((cache, None))
}

/// Resolve every enabled remote source a manifest declares. Failures on
/// never-cached sources are hard errors; refresh failures on cached
/// sources degrade to warnings.
pub fn sync_sources(env: &Env, manifest: &Manifest) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    for (name, decl) in &manifest.sources {
        if !decl.enabled {
            continue;
        }
        let Some(repo) = &decl.repo else {
            continue;
        };
        match sync(env, repo, &clone_url(repo)) {
            Ok((_, Some(warning))) => warnings.push(warning),
            Ok((_, None)) => {}
            Err(error) => {
                return Err(CoreError::GitFailed {
                    command: format!("resolving source '{name}' ({repo})"),
                    stderr: error.to_string(),
                });
            }
        }
    }
    Ok(warnings)
}

/// The cache's current HEAD, for freshness display.
pub fn cache_head(env: &Env, repo: &str) -> Option<String> {
    let cache = cache_dir(env, repo);
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(&cache)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;
    use std::fs;
    use std::path::Path;

    fn init_fixture_repo(dir: &Path) -> String {
        fs::create_dir_all(dir.join("skills/gh")).unwrap();
        fs::write(dir.join("skills/gh/SKILL.md"), "---\nname: gh\n---\nv1\n").unwrap();
        for args in [
            vec!["init", "--quiet", "-b", "main"],
            vec!["add", "."],
            vec![
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "--quiet",
                "-m",
                "one",
            ],
        ] {
            assert!(
                Command::new("git")
                    .args(&args)
                    .current_dir(dir)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        format!("file://{}", dir.display())
    }

    #[test]
    fn clone_then_fetch_reset_tracks_the_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let upstream = tmp.path().join("upstream");
        let url = init_fixture_repo(&upstream);

        let (cache, warning) = sync(&env, "owner/repo", &url).unwrap();
        assert!(warning.is_none());
        assert!(cache.join("skills/gh/SKILL.md").is_file());
        let first_head = cache_head(&env, "owner/repo").unwrap();

        // Upstream moves; sync fetches and hard-resets.
        fs::write(
            upstream.join("skills/gh/SKILL.md"),
            "---\nname: gh\n---\nv2\n",
        )
        .unwrap();
        assert!(
            Command::new("git")
                .args([
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "user.name=t",
                    "commit",
                    "-aqm",
                    "two"
                ])
                .current_dir(&upstream)
                .output()
                .unwrap()
                .status
                .success()
        );
        let (_, warning) = sync(&env, "owner/repo", &url).unwrap();
        assert!(warning.is_none());
        assert_ne!(cache_head(&env, "owner/repo").unwrap(), first_head);
        assert!(
            fs::read_to_string(cache.join("skills/gh/SKILL.md"))
                .unwrap()
                .contains("v2")
        );
    }

    #[test]
    fn refresh_failure_on_cached_clone_degrades_to_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let upstream = tmp.path().join("upstream");
        let url = init_fixture_repo(&upstream);
        sync(&env, "owner/repo", &url).unwrap();

        // The remote vanishes; the cached clone still serves.
        fs::remove_dir_all(&upstream).unwrap();
        let (cache, warning) = sync(&env, "owner/repo", &url).unwrap();
        assert!(warning.is_some());
        assert!(cache.join("skills/gh/SKILL.md").is_file());
    }

    #[test]
    fn never_cached_and_unreachable_is_a_hard_error() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let missing = format!("file://{}/nope", tmp.path().display());
        assert!(matches!(
            sync(&env, "owner/gone", &missing),
            Err(CoreError::GitFailed { .. })
        ));
    }

    #[test]
    fn shorthand_becomes_a_github_url_and_urls_pass_through() {
        assert_eq!(clone_url("a/b"), "https://github.com/a/b.git");
        assert_eq!(clone_url("https://x/y.git"), "https://x/y.git");
        assert_eq!(
            clone_url("git@github.com:a/b.git"),
            "git@github.com:a/b.git"
        );
    }
}
