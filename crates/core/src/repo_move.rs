//! The default catalog moved repositories (vanillagreencom/vstack →
//! vanillagreencom/kendex). A scope still naming the old repository is
//! planned as if it already named the new one, and the plan's own writes
//! record the move — the manifest in one write, the lock in one write. The
//! rewrite covers every place the repo string lives: leaving any one of
//! them behind makes the next plan read each installed package as
//! "installed from A but now set to come from B", a conflict per package.

use crate::lock::Lock;
use crate::manifest::{DEFAULT_SOURCE_REPO, LEGACY_SOURCE_REPO, Manifest};

/// The migration write's name in the plan preview.
pub const MOVE_DESCRIPTION: &str = "Point kendex at its new repository";

/// Whether a repo string names the moved repository, in any spelling a
/// manifest can carry: the `owner/repo` shorthand kendex seeds, or a full
/// clone URL written by hand (`clone_url` passes those through untouched).
/// The endings that say nothing about which repository it is — a trailing
/// slash, a `.git` suffix — are ignored, as the store's key already does.
pub fn names_old_default(repo: &str) -> bool {
    let repo = repo.trim_end_matches('/');
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    repo == LEGACY_SOURCE_REPO
        || repo == "https://github.com/vanillagreencom/vstack"
        || repo == "git@github.com:vanillagreencom/vstack"
}

fn rewrite(repo: &mut String, changed: &mut bool) {
    if names_old_default(repo) {
        *repo = DEFAULT_SOURCE_REPO.to_owned();
        *changed = true;
    }
}

/// The manifest with every old-default repo string rewritten — source
/// declarations and fork provenance. Source *names* stay: a scope that
/// declared `[sources.vstack]` keeps that name, only the repository it
/// points at moves. `None` when nothing names the old repository.
pub fn migrate_manifest(manifest: &Manifest) -> Option<Manifest> {
    let mut migrated = manifest.clone();
    let mut changed = false;
    for decl in migrated.sources.values_mut() {
        if let Some(repo) = decl.repo.as_mut() {
            rewrite(repo, &mut changed);
        }
    }
    for forks in migrated.forks.values_mut() {
        for provenance in forks.values_mut() {
            if let Some(repo) = provenance.repo.as_mut() {
                rewrite(repo, &mut changed);
            }
        }
    }
    changed.then_some(migrated)
}

/// The lock with every old-default repo string rewritten — the per-source
/// revision records and every entry's provenance. `None` when nothing
/// names the old repository.
pub fn migrate_lock(lock: &Lock) -> Option<Lock> {
    let mut migrated = lock.clone();
    let mut changed = false;
    for revision in migrated.sources.values_mut() {
        rewrite(&mut revision.repo, &mut changed);
    }
    for entry in migrated.entries.values_mut() {
        rewrite(&mut entry.source_repo, &mut changed);
    }
    changed.then_some(migrated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spelling_of_the_old_repository_is_recognized() {
        assert!(names_old_default("vanillagreencom/vstack"));
        assert!(names_old_default(
            "https://github.com/vanillagreencom/vstack"
        ));
        assert!(names_old_default(
            "https://github.com/vanillagreencom/vstack.git"
        ));
        assert!(names_old_default(
            "git@github.com:vanillagreencom/vstack.git"
        ));
        assert!(!names_old_default("vanillagreencom/kendex"));
        assert!(!names_old_default("someone/vstack"));
        // A substring is a different repository, never a match.
        assert!(!names_old_default("vanillagreencom/vstack-extras"));
    }

    #[test]
    fn a_manifest_without_the_old_repository_is_untouched() {
        assert_eq!(migrate_manifest(&crate::manifest::seed(&[])), None);
        assert_eq!(migrate_lock(&Lock::default()), None);
    }
}
