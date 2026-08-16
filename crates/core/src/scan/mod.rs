use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::harness::{HarnessAdapter, Surface, all_adapters};
use crate::model::{DetectedHarness, FileState, ItemKind, ObservedItem, Scope};
use crate::settings::AppSettings;

mod copilot;
mod entry;
pub use entry::RawEntry;
mod files;
mod hooks;
pub(crate) mod jsonc;
mod metadata;
mod plugins;
mod provenance;
mod readers;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub harnesses: Vec<DetectedHarness>,
    pub items: Vec<ObservedItem>,
    /// Registered projects whose directory is gone — flagged, never dropped.
    pub missing_projects: Vec<PathBuf>,
    /// Unreadable or unparsable surfaces; truth the scan could not reach.
    pub warnings: Vec<String>,
}

/// Read-only truth of this machine: every kind, every harness, global scope
/// plus every registered project.
pub fn scan(env: &Env, settings: &AppSettings) -> ScanResult {
    let mut scopes = vec![Scope::Global];
    scopes.extend(
        settings
            .projects
            .iter()
            .map(|p| Scope::Project { root: p.clone() }),
    );
    scan_scopes(env, &settings.harness_roots, &scopes)
}

/// The same engine over an explicit scope list — the CLI scans the current
/// project + global, the app scans everything registered.
pub fn scan_scopes(
    env: &Env,
    harness_roots: &std::collections::BTreeMap<String, PathBuf>,
    scopes: &[Scope],
) -> ScanResult {
    let mut result = ScanResult {
        harnesses: Vec::new(),
        items: Vec::new(),
        missing_projects: Vec::new(),
        warnings: Vec::new(),
    };
    let mut provenance = provenance::OriginCache::default();

    for scope in scopes {
        match scope {
            Scope::Global => {
                for adapter in all_adapters() {
                    let root = harness_roots
                        .get(adapter.id().name())
                        .cloned()
                        .unwrap_or_else(|| adapter.default_global_root(env));
                    if let Some(found) = adapter.detect(env, &root) {
                        result.harnesses.push(found);
                    }
                    for kind in ItemKind::ALL {
                        for surface in adapter.global_surfaces(kind, &root, env) {
                            scan_surface(
                                adapter,
                                kind,
                                Scope::Global,
                                &surface,
                                env,
                                &mut provenance,
                                &mut result,
                            );
                        }
                    }
                }
            }
            Scope::Project { root: project } => {
                if !project.is_dir() {
                    result.missing_projects.push(project.clone());
                    continue;
                }
                for adapter in all_adapters() {
                    for kind in ItemKind::ALL {
                        for surface in adapter.project_surfaces(kind, project, env) {
                            scan_surface(
                                adapter,
                                kind,
                                scope.clone(),
                                &surface,
                                env,
                                &mut provenance,
                                &mut result,
                            );
                        }
                    }
                }
            }
        }
    }

    result
}

fn scan_surface(
    adapter: &dyn HarnessAdapter,
    kind: ItemKind,
    scope: Scope,
    surface: &Surface,
    env: &Env,
    provenance: &mut provenance::OriginCache,
    result: &mut ScanResult,
) {
    match surface {
        Surface::FileDir { dir, exts, prefix } => {
            for found in files::scan_file_dir(dir, exts, *prefix) {
                warn_unknown_tags(&found, result);
                result.items.push(ObservedItem {
                    kind,
                    name: found.name,
                    harness: adapter.id(),
                    scope: scope.clone(),
                    file_state: files::state_of(&found.path),
                    origin: provenance.origin_of(&found.path),
                    path: found.path,
                    enabled: Some(found.enabled),
                    tags: found.meta.tags,
                    description: found.meta.description,
                    modified_at: found.modified_at,
                });
            }
        }
        Surface::SubdirPerItem { dir, marker } => {
            for found in files::scan_subdirs(dir, marker) {
                warn_unknown_tags(&found, result);
                result.items.push(ObservedItem {
                    kind,
                    name: found.name,
                    harness: adapter.id(),
                    scope: scope.clone(),
                    file_state: files::state_of(&found.path),
                    origin: provenance.origin_of(&found.path),
                    path: found.path,
                    enabled: Some(found.enabled),
                    tags: found.meta.tags,
                    description: found.meta.description,
                    modified_at: found.modified_at,
                });
            }
        }
        Surface::Structured { path, reader } => {
            if path.exists() {
                scan_structured_file(adapter, kind, &scope, path, reader, env, result);
            }
        }
        Surface::StructuredDir { dir, ext, reader } => {
            for path in files::scan_documents(dir, ext) {
                scan_structured_file(adapter, kind, &scope, &path, reader, env, result);
            }
        }
    }
}

// A tag nobody recognises is almost always a near-miss for one that
// exists — `tests` for `testing`. Silently dropping it leaves the author
// thinking the item is tagged when it is not, so the scan says so and names
// the vocabulary.
fn warn_unknown_tags(found: &files::FoundFile, result: &mut ScanResult) {
    let Some(message) = found.meta.unknown_warning() else {
        return;
    };
    // One file read through two tools' folders is one mistake, not two: the
    // paths differ (each tool links to the shared folder its own way) and
    // the complaint is identical, so it is said once.
    let warning = format!("{}: {message}", found.path.display());
    if !result.warnings.contains(&warning) {
        result.warnings.push(warning);
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_structured_file(
    adapter: &dyn HarnessAdapter,
    kind: ItemKind,
    scope: &Scope,
    path: &std::path::Path,
    reader: &crate::harness::Reader,
    env: &Env,
    result: &mut ScanResult,
) {
    match readers::read_structured(path, reader, env) {
        Ok(entries) => {
            for entry in entries {
                result.items.push(ObservedItem {
                    kind,
                    name: entry.name,
                    harness: adapter.id(),
                    scope: scope.clone(),
                    file_state: match entry.source_path {
                        Some(_) => FileState::Dir,
                        None => FileState::ConfigEntry,
                    },
                    path: entry.source_path.unwrap_or_else(|| path.to_path_buf()),
                    enabled: entry.enabled,
                    origin: None,
                    // Nothing to read: a structured reader hands back an
                    // entry, not a document, and the entry's own files (a
                    // plugin's directory) are not in a format with a header.
                    tags: Vec::new(),
                    description: entry.description,
                    // One file holds every entry of this kind; its mtime
                    // would describe all of them at once, not this one.
                    modified_at: None,
                });
            }
        }
        Err(message) => result
            .warnings
            .push(format!("{}: {message}", path.display())),
    }
}

#[cfg(test)]
mod tests;
