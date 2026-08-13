use std::path::PathBuf;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::Manifest;
use crate::process::Hardened;

/// `owner/repo` → clone URL. Full URLs pass through untouched;
/// `VSTACK_GIT_BASE` rebases shorthands onto another host (test fixtures).
pub fn clone_url(env: &Env, repo: &str) -> String {
    if repo.contains("://") || repo.starts_with("git@") {
        return repo.to_owned();
    }
    match env.var("VSTACK_GIT_BASE") {
        Some(base) => format!("{}/{repo}", base.trim_end_matches('/')),
        None => format!("https://github.com/{repo}.git"),
    }
}

pub fn cache_dir(env: &Env, repo: &str) -> PathBuf {
    env.source_cache_dir().join(repo.replace('/', "_"))
}

fn run(git: Hardened) -> Result<()> {
    let command = git.label().to_owned();
    let output = git.run()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CoreError::GitFailed {
            command,
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
        // Pinned: the cached repository's own config must not decide where
        // these two write.
        let refreshed =
            run(Hardened::git_in(&cache, &["fetch", "origin", "--quiet"])).and_then(|()| {
                run(Hardened::git_in(
                    &cache,
                    &["reset", "--hard", "origin/HEAD", "--quiet"],
                ))
            });
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
    run(Hardened::git(
        &["clone", "--quiet", url, &cache.display().to_string()],
        None,
    ))?;
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
        match sync(env, repo, &clone_url(env, repo)) {
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
    let output = Hardened::git_in(&cache, &["rev-parse", "--short", "HEAD"])
        .run()
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
                Hardened::git(&args, Some(dir))
                    .run()
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
            Hardened::git(
                &[
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "user.name=t",
                    "commit",
                    "-aqm",
                    "two"
                ],
                Some(&upstream)
            )
            .run()
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
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        assert_eq!(clone_url(&env, "a/b"), "https://github.com/a/b.git");
        assert_eq!(clone_url(&env, "https://x/y.git"), "https://x/y.git");
        assert_eq!(
            clone_url(&env, "git@github.com:a/b.git"),
            "git@github.com:a/b.git"
        );
        let rebased = env.with_var("VSTACK_GIT_BASE", "file:///fixtures/");
        assert_eq!(clone_url(&rebased, "a/b"), "file:///fixtures/a/b");
        assert_eq!(clone_url(&rebased, "https://x/y.git"), "https://x/y.git");
    }
}
