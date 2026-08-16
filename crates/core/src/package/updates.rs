//! Which installed packages have newer versions, computed from the mirrors
//! alone — offline, read-only, no plan. Ignoring a package's updates is a
//! machine-local preference in settings.toml, never manifest intent: a
//! notification choice committed to a shared repository would silence a
//! whole team.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::error::Result;
use crate::model::{ItemKind, Scope};
use crate::remote::history;
use crate::settings;

/// One version a row points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionRef {
    pub commit: String,
    /// Release name when a tag points at the commit.
    pub label: Option<String>,
    pub date: Option<String>,
}

/// One declared package's update standing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRow {
    pub scope: Scope,
    pub kind: ItemKind,
    pub name: String,
    pub source: String,
    pub repo: String,
    /// The content revision installed now, when the lock records it.
    pub current: Option<VersionRef>,
    /// The newest content revision the mirror knows.
    pub latest: Option<VersionRef>,
    /// The package's files changed between current and latest — a moved
    /// repository that never touched this package is not an update.
    pub update_available: bool,
    /// Held at a version (`rev` on the declaration) — manual updates.
    pub pinned: bool,
    /// The user asked to stop hearing about this package's updates.
    pub ignored: bool,
    /// The installed files were edited by hand; updating is blocked until
    /// the edit is kept as a fork or discarded.
    pub blocked_by_local_edit: bool,
    /// This package is a local fork of a catalog item.
    pub forked: bool,
    /// Installations of this package disagree on their source commit.
    pub mixed: bool,
}

/// A package whose update notifications are switched off, by everything
/// that identifies it: an ignore for one project's `gh` skill from one
/// repository must not silence another project's unrelated `gh`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct IgnoredUpdate {
    /// `"global"` or the project root path.
    pub scope: String,
    pub kind: ItemKind,
    pub name: String,
    pub repo: String,
}

pub(crate) fn scope_key(scope: &Scope) -> String {
    match scope.canonical() {
        Scope::Global => "global".to_owned(),
        Scope::Project { root } => root.display().to_string(),
    }
}

/// Every declared remote package's standing, ignored rows included — they
/// come back flagged, never filtered, so the surface that hides them can
/// also offer the way back.
pub fn updates(env: &Env, scope: &Scope) -> Result<Vec<UpdateRow>> {
    let Some(manifest) = load_current(env, scope)? else {
        return Ok(Vec::new());
    };
    let lock = crate::lock::load_file(&crate::lock::lock_path(env, scope))?;
    let lock = match lock {
        crate::lock::LockFile::Current(lock) => lock,
        _ => crate::lock::Lock::default(),
    };
    let ignored = settings::load(env)?.ignored_updates;
    let key = scope_key(scope);
    let mut rows = Vec::new();
    for kind in ItemKind::ALL {
        for name in manifest.declared(kind).keys() {
            let forked = manifest
                .forks
                .get(&kind)
                .is_some_and(|forks| forks.contains_key(name));
            let Ok(package) = crate::package::package_ref(env, scope, &manifest, kind, name) else {
                // Path and local sources have no versions; a pending remote
                // has no mirror to ask. Neither is an update row — except a
                // fork, which the Library still needs to know about: it
                // rides along with no versions and no update.
                if forked {
                    rows.push(fork_row(scope, kind, name, &manifest));
                }
                continue;
            };
            let log = history::subtree_log(&package.mirror, &package.tip, &package.subtree);
            let refer = |commit: &str| VersionRef {
                label: log
                    .iter()
                    .find(|row| row.commit == commit)
                    .and_then(|row| row.tags.first().cloned()),
                date: log
                    .iter()
                    .find(|row| row.commit == commit)
                    .map(|row| row.date.clone()),
                commit: commit.to_owned(),
            };
            let latest = log.first().map(|row| refer(&row.commit));
            let commits: Vec<String> = lock
                .entries
                .values()
                .filter(|entry| entry.kind == kind && &entry.name == name)
                .filter_map(|entry| entry.source_commit.clone())
                .collect();
            let mixed = commits.windows(2).any(|pair| pair[0] != pair[1]);
            let current = commits
                .last()
                .and_then(|commit| {
                    history::last_content_commit(&package.mirror, commit, &package.subtree)
                })
                .map(|commit| refer(&commit));
            let update_available = match (&current, &latest) {
                (Some(current), Some(latest)) => current.commit != latest.commit,
                // Nothing installed yet, or a lock predating the record:
                // there is nothing to honestly compare, and a false "update"
                // on every legacy install would drown the real ones.
                _ => false,
            };
            rows.push(UpdateRow {
                scope: scope.clone(),
                kind,
                name: name.clone(),
                source: package.source_name.clone(),
                pinned: manifest
                    .declared(kind)
                    .get(name)
                    .is_some_and(|decl| decl.rev.is_some()),
                ignored: ignored.iter().any(|entry| {
                    entry.scope == key
                        && entry.kind == kind
                        && &entry.name == name
                        && entry.repo == package.repo
                }),
                blocked_by_local_edit: lock
                    .entries
                    .values()
                    .filter(|entry| entry.kind == kind && &entry.name == name)
                    .any(|entry| crate::engine::edited_on_disk(env, scope, entry)),
                forked,
                repo: package.repo,
                current,
                latest,
                update_available,
                mixed,
            });
        }
    }
    Ok(rows)
}

/// A fork's row: no versions, no update — the Library still needs to
/// know it is a fork.
fn fork_row(
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    manifest: &crate::manifest::Manifest,
) -> UpdateRow {
    UpdateRow {
        scope: scope.clone(),
        kind,
        name: name.to_owned(),
        source: manifest
            .declared(kind)
            .get(name)
            .map(|decl| decl.source.clone())
            .unwrap_or_default(),
        repo: String::new(),
        current: None,
        latest: None,
        update_available: false,
        pinned: false,
        ignored: false,
        blocked_by_local_edit: false,
        forked: true,
        mixed: false,
    }
}

/// Switch one package's update notifications off or on. A settings write
/// and nothing else: no plan runs, nothing installs, nothing is touched
/// but the preference.
pub fn set_ignored(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    repo: &str,
    ignored: bool,
) -> Result<()> {
    let mut current = settings::load(env)?;
    let entry = IgnoredUpdate {
        scope: scope_key(scope),
        kind,
        name: name.to_owned(),
        repo: repo.to_owned(),
    };
    current
        .ignored_updates
        .retain(|existing| existing != &entry);
    if ignored {
        current.ignored_updates.push(entry);
        current.ignored_updates.sort_by(|a, b| {
            (&a.scope, a.kind.name(), &a.name).cmp(&(&b.scope, b.kind.name(), &b.name))
        });
    }
    settings::save(env, &current)
}

fn load_current(env: &Env, scope: &Scope) -> Result<Option<crate::manifest::Manifest>> {
    match crate::manifest::load(&crate::manifest::manifest_path(env, scope))? {
        crate::manifest::ManifestFile::Current(manifest) => Ok(Some(*manifest)),
        _ => Ok(None),
    }
}
