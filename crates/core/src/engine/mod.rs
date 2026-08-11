use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::apply::{Op, Plan, PlannedOp, Pre};
use crate::env::Env;
use crate::error::Result;
use crate::lock::{Lock, lock_path};
use crate::manifest::{self, Manifest, ManifestFile};
use crate::model::{HarnessId, ItemKind, Scope};

pub mod adopt;
pub mod desired;
mod item_plan;
pub mod ops;

use item_plan::plan_item;

use desired::{Artifact, Desired, desired_state};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum DriftState {
    /// Declared but not on disk (or never recorded).
    Missing,
    /// On disk but no longer matching declaration + source.
    Stale,
    /// Recorded in the lock but no longer declared.
    Orphaned,
    /// On disk in a managed surface, but not ours.
    Unmanaged,
    /// Needs a human: foreign symlink, occupied target, or provenance clash.
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DriftRow {
    pub kind: ItemKind,
    pub name: String,
    pub harness: HarnessId,
    pub scope: Scope,
    pub state: DriftState,
    pub detail: String,
}

#[derive(Debug)]
pub struct EngineReport {
    pub drift: Vec<DriftRow>,
    pub plan: Plan,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PlanOptions {
    /// Remove orphaned (locked-but-undeclared) artifacts. Refresh keeps
    /// them (v1 semantics); reconcile and `remove` clean them up.
    pub remove_orphans: bool,
    /// Restrict orphan removal to these names (the `remove` verb).
    pub removal_filter: Option<Vec<String>>,
}

/// Compute drift and the plan that would fix it, in one pass — the Audit
/// page and `apply` both consume this.
pub fn plan_scope(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    lock: &Lock,
    options: &PlanOptions,
) -> Result<EngineReport> {
    let state = desired_state(env, scope, manifest, lock)?;
    let mut drift = Vec::new();
    let mut ops: Vec<PlannedOp> = Vec::new();
    let mut new_lock = Lock {
        version: crate::lock::LOCK_VERSION,
        entries: BTreeMap::new(),
    };
    let mut written_canonicals: BTreeSet<PathBuf> = BTreeSet::new();

    if let Some(updated) = &state.manifest_update {
        ops.push(PlannedOp {
            description: "merge upstream skill additions into vstack.toml".into(),
            op: Op::WriteManifest {
                path: manifest::manifest_path(env, scope),
                manifest: Box::new(updated.clone()),
            },
        });
    }

    for item in &state.items {
        plan_item(
            item,
            scope,
            lock,
            &mut drift,
            &mut ops,
            &mut new_lock,
            &mut written_canonicals,
        )?;
    }

    orphans(
        env,
        scope,
        manifest,
        lock,
        &state.items,
        options,
        &mut drift,
        &mut ops,
        &mut new_lock,
    );

    if new_lock.entries != lock.entries {
        ops.push(PlannedOp {
            description: "update lock".into(),
            op: Op::WriteLock {
                path: lock_path(env, scope),
                lock: Box::new(new_lock),
            },
        });
    }

    let mut report = EngineReport {
        drift,
        plan: Plan {
            scope: scope.clone(),
            ops,
        },
        notes: state.notes,
    };
    unmanaged_rows(env, scope, manifest, lock, &state.items, &mut report.drift);
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn orphans(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    lock: &Lock,
    desired: &[Desired],
    options: &PlanOptions,
    drift: &mut Vec<DriftRow>,
    ops: &mut Vec<PlannedOp>,
    new_lock: &mut Lock,
) {
    let desired_keys: BTreeSet<&String> = desired.iter().map(|d| &d.key).collect();
    let keep_canonical: BTreeSet<PathBuf> = desired
        .iter()
        .filter_map(|d| match &d.artifact {
            Artifact::Tree { canonical, .. } => Some(canonical.clone()),
            Artifact::File { .. } => None,
        })
        .collect();
    let mut trashed: BTreeSet<PathBuf> = BTreeSet::new();

    for (key, entry) in &lock.entries {
        if desired_keys.contains(key) {
            continue;
        }
        // Declared but skipped this pass (pending/disabled source, missing
        // from source): keep the record, it is not an orphan.
        if manifest.declared(entry.kind).contains_key(&entry.name) {
            new_lock.entries.insert(key.clone(), entry.clone());
            continue;
        }
        let removable = options.remove_orphans
            && options
                .removal_filter
                .as_ref()
                .is_none_or(|names| names.contains(&entry.name));
        drift.push(DriftRow {
            kind: entry.kind,
            name: entry.name.clone(),
            harness: entry.harness,
            scope: scope.clone(),
            state: DriftState::Orphaned,
            detail: if removable {
                "no longer declared — will be removed".into()
            } else {
                "recorded in lock but no longer declared".into()
            },
        });
        if !removable {
            new_lock.entries.insert(key.clone(), entry.clone());
            continue;
        }
        for path in ops::artifact_paths(env, scope, entry) {
            let shared = keep_canonical.contains(&path);
            if shared || !trashed.insert(path.clone()) {
                continue;
            }
            if path.exists() || path.is_symlink() {
                ops.push(PlannedOp {
                    description: format!("trash {}", path.display()),
                    op: Op::Trash {
                        path,
                        pre: Pre::Any,
                    },
                });
            }
        }
    }
}

fn unmanaged_rows(
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
    let declared_names: BTreeSet<(ItemKind, &String)> = manifest
        .agents
        .keys()
        .map(|n| (ItemKind::Agent, n))
        .chain(manifest.skills.keys().map(|n| (ItemKind::Skill, n)))
        .collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for item in &scan.items {
        if !matches!(item.kind, ItemKind::Agent | ItemKind::Skill) {
            continue;
        }
        let key = crate::lock::entry_key(item.kind, &item.name, item.harness);
        if known.contains(&key)
            || declared_names.contains(&(item.kind, &item.name))
            || !seen.insert(key)
        {
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

/// Read-only audit for a scope. A legacy or absent manifest still reports
/// unmanaged items; nothing is planned that would touch a legacy file.
pub fn audit(env: &Env, scope: &Scope) -> Result<EngineReport> {
    let manifest_file = manifest::load(&manifest::manifest_path(env, scope))?;
    let lock = crate::lock::load(&lock_path(env, scope))?;
    match manifest_file {
        ManifestFile::Current(manifest) => {
            plan_scope(env, scope, &manifest, &lock, &PlanOptions::default())
        }
        other => {
            let mut report = EngineReport {
                drift: Vec::new(),
                plan: Plan {
                    scope: scope.clone(),
                    ops: Vec::new(),
                },
                notes: Vec::new(),
            };
            if matches!(other, ManifestFile::Legacy { .. }) {
                report
                    .notes
                    .push("v1 manifest — read-only until migration (Phase 6 importer)".into());
            }
            let empty = Manifest::default();
            unmanaged_rows(env, scope, &empty, &lock, &[], &mut report.drift);
            Ok(report)
        }
    }
}
