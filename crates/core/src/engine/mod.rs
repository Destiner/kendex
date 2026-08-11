use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::apply::{Op, Plan, PlannedOp};
use crate::env::Env;
use crate::error::Result;
use crate::lock::{Lock, lock_path};
use crate::manifest::{self, Manifest, ManifestFile};
use crate::model::{HarnessId, ItemKind, Scope};

pub mod adopt;
pub mod desired;
mod desired_agent;
mod desired_kinds;
mod item_plan;
pub mod ops;
mod removal;
mod targets;

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
        let path = manifest::manifest_path(env, scope);
        ops.push(PlannedOp {
            description: "merge upstream skill additions into vstack.toml".into(),
            op: Op::WriteManifest {
                pre: crate::apply::Pre::observed(&path)?,
                path,
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
        &state,
        options,
        &mut drift,
        &mut ops,
        &mut new_lock,
    )?;

    if new_lock.entries != lock.entries {
        let path = lock_path(env, scope);
        ops.push(PlannedOp {
            description: "update lock".into(),
            op: Op::WriteLock {
                pre: crate::apply::Pre::observed(&path)?,
                path,
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
    state: &desired::DesiredState,
    options: &PlanOptions,
    drift: &mut Vec<DriftRow>,
    ops: &mut Vec<PlannedOp>,
    new_lock: &mut Lock,
) -> Result<()> {
    let desired = &state.items;
    let desired_keys: BTreeSet<&String> = desired.iter().map(|d| &d.key).collect();
    let keep_canonical: BTreeSet<PathBuf> = desired
        .iter()
        .filter_map(|d| match &d.artifact {
            Artifact::Tree { canonical, .. } => Some(canonical.clone()),
            Artifact::File { .. } | Artifact::Registration { .. } => None,
        })
        .collect();
    let mut trashed: BTreeSet<PathBuf> = BTreeSet::new();

    for (key, entry) in &lock.entries {
        if desired_keys.contains(key) {
            continue;
        }
        // Declared but skipped this pass (pending/disabled source, missing
        // from source): keep the record, it is not an orphan. A declaration
        // that did resolve has already said everything it wants installed,
        // so an entry it did not ask for — a harness dropped from its list —
        // is stranded and must be cleaned up like any other orphan.
        let unreachable_source = manifest.declared(entry.kind).contains_key(&entry.name)
            && !state.processed.contains(&(entry.kind, entry.name.clone()));
        if unreachable_source {
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
        for planned in removal::removal_ops(env, scope, entry)? {
            // A skill tree another installation still wants stays put:
            // shared physical targets are reference-counted, not deleted.
            if let Op::Trash { path, .. } = &planned.op
                && (keep_canonical.contains(path) || !trashed.insert(path.clone()))
            {
                continue;
            }
            ops.push(planned);
        }
    }
    Ok(())
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
