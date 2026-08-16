use std::path::PathBuf;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::Manifest;

pub mod history;
pub mod store;

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

/// One repository declaration, answered: the commit it names right now and
/// the directory holding that commit's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub commit: String,
    pub root: PathBuf,
    /// Set when the cache answered something the network could not confirm.
    pub warning: Option<String>,
}

impl Resolution {
    fn at(commit: &str, root: PathBuf) -> Resolution {
        Resolution {
            commit: commit.to_owned(),
            root,
            warning: None,
        }
    }
}

/// Bring a declaration up to date: fetch the mirror, re-resolve a tracking
/// selector, and publish the commit's checkout if the cache lacks it.
///
/// A pin the cache already holds skips the network entirely — that is what
/// makes a pinned install work offline. A pin the cache lacks and the
/// network cannot supply is a hard error naming the pin. A tracking
/// selector whose fetch fails degrades to a warning and the cached commit,
/// so an offline session keeps working.
pub fn sync(env: &Env, repo: &str, rev: Option<&str>) -> Result<Resolution> {
    let url = clone_url(env, repo);
    let key = store::repo_key(&url);
    let mirror = store::mirror_dir(env, &key);

    if let Some(pin) = rev.filter(|rev| store::is_pin(rev)) {
        if let Some(root) = store::published(env, &key, pin) {
            return Ok(Resolution::at(pin, root));
        }
        let _guard = store::lock_repo(env, &key)?;
        let fetched = store::ensure_mirror(&mirror, &url).and_then(|()| store::fetch(&mirror));
        if !store::has_commit(&mirror, pin) {
            return Err(CoreError::PinUnavailable {
                repo: repo.to_owned(),
                pin: pin.to_owned(),
                reason: match fetched {
                    Err(error) => error.to_string(),
                    Ok(()) => "no such commit in the repository".to_owned(),
                },
            });
        }
        return Ok(Resolution::at(
            pin,
            store::publish(env, &key, &mirror, pin)?,
        ));
    }

    let selector = rev.unwrap_or("HEAD");
    let _guard = store::lock_repo(env, &key)?;
    let warning = match store::ensure_mirror(&mirror, &url).and_then(|()| store::fetch(&mirror)) {
        Ok(()) => None,
        Err(error) => Some(format!("{repo}: using cached version ({error})")),
    };
    let Some(commit) = store::resolve_ref(&mirror, selector) else {
        // The mirror cannot name this selector: nothing was fetched, or the
        // branch or tag is gone upstream. The pre-2.0 clone is the last
        // thing left that might hold content.
        return match legacy_resolution(env, repo) {
            Some(resolution) => Ok(resolution),
            None => Err(CoreError::PinUnavailable {
                repo: repo.to_owned(),
                pin: selector.to_owned(),
                reason: warning.unwrap_or_else(|| "no such branch or tag".to_owned()),
            }),
        };
    };
    let root = match store::published(env, &key, &commit) {
        Some(root) => root,
        None => store::publish(env, &key, &mirror, &commit)?,
    };
    Ok(Resolution {
        commit,
        root,
        warning,
    })
}

/// What the cache can answer for a declaration without any network. The
/// read path: planning, rendering, and every listing go through this, so a
/// refresh in another window is never a precondition for reading what is
/// already installed.
pub fn cached(env: &Env, repo: &str, rev: Option<&str>) -> Result<Option<Resolution>> {
    let key = store::repo_key(&clone_url(env, repo));
    let mirror = store::mirror_dir(env, &key);
    let selector = rev.unwrap_or("HEAD");
    let commit = match store::is_pin(selector) {
        true => Some(selector.to_owned()),
        false => store::resolve_ref(&mirror, selector),
    };
    if let Some(commit) = commit {
        if let Some(root) = store::published(env, &key, &commit) {
            return Ok(Some(Resolution::at(&commit, root)));
        }
        // The mirror holds the objects even when the checkout is missing or
        // no longer matches what was published: rebuilding it is local.
        if store::has_commit(&mirror, &commit) {
            match store::lock_repo(env, &key) {
                Ok(_guard) => {
                    let root = store::publish(env, &key, &mirror, &commit)?;
                    return Ok(Some(Resolution::at(&commit, root)));
                }
                // Someone else is materializing this repository. Reading is
                // never worth failing a whole scope over: this one source
                // reads as unfetched, like any other content that is not
                // here yet, and the next pass picks it up.
                Err(CoreError::CacheBusy { .. }) => {}
                Err(error) => return Err(error),
            }
        }
    }
    // A pin is answered by that commit or not at all. The pre-2.0 clone
    // sits on whatever it last reset to, which is a different commit's
    // content under a name that promised one.
    match store::is_pin(selector) {
        true => Ok(None),
        false => Ok(legacy_resolution(env, repo)),
    }
}

/// The v0.1 mutable clone, read where the new layout has nothing yet: an
/// offline first run after an update still resolves. Nothing writes to it
/// and nothing deletes it — the first successful refresh publishes a
/// per-commit checkout and this stops being consulted.
fn legacy_resolution(env: &Env, repo: &str) -> Option<Resolution> {
    let legacy = store::legacy_clone(env, repo);
    store::legacy_head(&legacy).map(|commit| Resolution {
        commit,
        root: legacy,
        warning: Some(format!(
            "{repo}: reading the pre-2.0 cache; the next refresh replaces it"
        )),
    })
}

/// Fetch every enabled remote source's mirror, pins included. `sync`
/// deliberately skips the network for a cached pin — that is what makes a
/// pinned install work offline — so update discovery needs its own fetch:
/// a repository someone pinned still grows new versions worth knowing
/// about. Failures degrade to warnings; discovery is never worth failing
/// a scope over.
pub fn fetch_all(env: &Env, manifest: &Manifest) -> Vec<String> {
    let mut warnings = Vec::new();
    for decl in manifest.sources.values() {
        if !decl.enabled {
            continue;
        }
        let Some(repo) = &decl.repo else {
            continue;
        };
        let url = clone_url(env, repo);
        let key = store::repo_key(&url);
        let mirror = store::mirror_dir(env, &key);
        let guard = match store::lock_repo(env, &key) {
            Ok(guard) => guard,
            Err(error) => {
                warnings.push(format!("{repo}: not checked ({error})"));
                continue;
            }
        };
        if let Err(error) = store::ensure_mirror(&mirror, &url).and_then(|()| store::fetch(&mirror))
        {
            warnings.push(format!("{repo}: not checked ({error})"));
        }
        drop(guard);
    }
    warnings
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
        match sync(env, repo, decl.rev.as_deref()) {
            Ok(resolution) => warnings.extend(resolution.warning),
            // Already names the repository and the pin the user must fix.
            Err(error @ CoreError::PinUnavailable { .. }) => return Err(error),
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

/// The commit a source reads as right now, abbreviated for display. Cheap
/// on purpose: no checkout is verified and nothing is materialized.
pub fn cache_head(env: &Env, repo: &str, rev: Option<&str>) -> Option<String> {
    let key = store::repo_key(&clone_url(env, repo));
    let commit = match rev.filter(|rev| store::is_pin(rev)) {
        Some(pin) => pin.to_owned(),
        None => store::resolve_ref(&store::mirror_dir(env, &key), rev.unwrap_or("HEAD"))
            .or_else(|| store::legacy_head(&store::legacy_clone(env, repo)))?,
    };
    Some(commit.chars().take(7).collect())
}

#[cfg(test)]
mod tests;
