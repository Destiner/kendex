use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::env::Env;
use crate::lock::Lock;
use crate::manifest::Manifest;
use crate::model::{ItemKind, Scope};

use super::desired::{self, Desired};
use super::{DriftRow, DriftState};

pub(super) fn unmanaged_rows(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    lock: &Lock,
    desired: &[Desired],
    drift: &mut Vec<DriftRow>,
) {
    let scan = crate::scan::scan_scopes(env, &BTreeMap::new(), std::slice::from_ref(scope));
    let known: BTreeSet<String> = desired
        .iter()
        .map(|d| d.key.clone())
        .chain(lock.entries.keys().cloned())
        .collect();
    let declared_keys = declared_installation_keys(manifest, scope);
    let mut owned: BTreeSet<PathBuf> = desired
        .iter()
        .flat_map(|d| desired::artifact_paths(&d.artifact))
        .collect();
    owned.extend(declared_artifact_paths(env, scope, manifest));
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for item in &scan.items {
        if !matches!(item.kind, ItemKind::Agent | ItemKind::Skill) || owned.contains(&item.path) {
            continue;
        }
        let key = crate::lock::entry_key(item.kind, &item.name, item.harness);
        if known.contains(&key) || declared_keys.contains(&key) || !seen.insert(key) {
            continue;
        }
        drift.push(DriftRow {
            kind: item.kind,
            name: item.name.clone(),
            harness: item.harness,
            scope: scope.clone(),
            state: DriftState::Unmanaged,
            detail: item.path.display().to_string(),
        });
    }
}

/// Every installation the manifest asks for, by lock key. A declaration
/// speaks only for the harnesses it targets: a same-named item in a harness
/// it does not target is someone else's, and hiding it would leave it
/// loading forever with no drift row to discover it by.
fn declared_installation_keys(manifest: &Manifest, scope: &Scope) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (kind, table) in [
        (ItemKind::Agent, &manifest.agents),
        (ItemKind::Skill, &manifest.skills),
    ] {
        for (name, decl) in table {
            for harness in desired::target_harnesses(decl, manifest, kind, scope) {
                keys.insert(crate::lock::entry_key(kind, name, harness));
            }
        }
    }
    keys
}

/// Where those installations live, whether or not this pass could build
/// them. Skills share one canonical tree across harnesses, so the path is
/// what says "ours", not the harness the scanner attributes it to.
fn declared_artifact_paths(env: &Env, scope: &Scope, manifest: &Manifest) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::new();
    for (kind, table) in [
        (ItemKind::Agent, &manifest.agents),
        (ItemKind::Skill, &manifest.skills),
    ] {
        for (name, decl) in table {
            paths.extend(desired::declared_paths(
                env, scope, manifest, kind, name, decl,
            ));
        }
    }
    paths
}
