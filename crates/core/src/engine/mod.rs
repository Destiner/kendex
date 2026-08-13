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
mod catalog;
mod config_edits;
mod copilot;
pub mod desired;
mod desired_agent;
mod desired_command;
mod desired_kinds;
mod desired_mcp;
mod desired_skill;
mod desired_source;
mod gemini;
mod item_plan;
pub mod ops;
mod removal;
mod scope_writes;
mod targets;
mod tree_plan;
mod unmanaged;

use item_plan::plan_item;
use scope_writes::{
    plan_config_edits, plan_lock_write, plan_schema_upgrade, plan_settings_seed, source_revisions,
};
use unmanaged::unmanaged_rows;

use desired::desired_state;

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

/// A per-item render or parse warning, with the fix when there is one —
/// shown in plan previews, the CLI, and the Audit page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ItemWarning {
    pub kind: ItemKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<HarnessId>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug)]
pub struct EngineReport {
    pub drift: Vec<DriftRow>,
    pub plan: Plan,
    pub notes: Vec<String>,
    pub warnings: Vec<ItemWarning>,
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
    // Identity first: every derived path and the eventual scope lock key
    // off the canonical root, whatever spelling the caller passed.
    let scope = &scope.canonical();
    let state = desired_state(env, scope, manifest, lock)?;
    let mut drift = Vec::new();
    let mut ops: Vec<PlannedOp> = Vec::new();
    let mut new_lock = Lock {
        version: crate::lock::LOCK_VERSION,
        entries: BTreeMap::new(),
        sources: source_revisions(manifest, lock, &state),
    };
    let mut written = tree_plan::Written::default();
    let mut config_edits = config_edits::ConfigEditPlan::default();

    if let Some(updated) = &state.manifest_update {
        let path = manifest::manifest_path(env, scope);
        let mut updated = updated.clone();
        updated.schema = manifest::MANIFEST_SCHEMA;
        ops.push(PlannedOp {
            description: "Add new catalog skills to vstack.toml".into(),
            op: Op::WriteManifest {
                pre: crate::apply::Pre::observed(&path)?,
                path,
                manifest: Box::new(updated),
            },
        });
    } else if manifest.schema < manifest::MANIFEST_SCHEMA {
        plan_schema_upgrade(env, scope, manifest, &mut ops)?;
    }

    // What earlier installs put on disk under another kind's name. A path
    // one of them wrote is ours to replace, whichever entry holds it now.
    let emitted_paths: BTreeSet<PathBuf> = lock
        .entries
        .values()
        .filter_map(|entry| entry.emitted.as_ref())
        .flat_map(|emitted| emitted.paths.iter().cloned())
        .collect();

    for item in &state.items {
        let mut sink = item_plan::PlanSink {
            drift: &mut drift,
            ops: &mut ops,
            config_edits: &mut config_edits,
            new_lock: &mut new_lock,
            written: &mut written,
        };
        plan_item(item, scope, lock, &emitted_paths, &mut sink)?;
    }

    plan_settings_seed(scope, &state, &mut ops, &mut drift)?;

    // Trash ops all pass one guard: writes for this pass are already
    // planned, so anything still wanted is known, and no path goes to the
    // trash twice.
    let mut guard = removal::TrashGuard::new(&state.items);
    removal::stale_emitted(&state, lock, &mut guard, &mut ops);

    let refused_keys = plan_refusals(
        env,
        scope,
        lock,
        &state,
        &mut guard,
        &mut drift,
        &mut ops,
        &mut config_edits,
    )?;

    removal::orphans(
        env,
        scope,
        manifest,
        lock,
        &state,
        options,
        &refused_keys,
        &mut guard,
        &mut drift,
        &mut ops,
        &mut config_edits,
        &mut new_lock,
    )?;

    plan_config_edits(config_edits, &mut ops)?;
    plan_lock_write(env, scope, lock, new_lock, &mut ops)?;

    let mut report = EngineReport {
        drift,
        plan: Plan {
            scope: scope.clone(),
            ops,
        },
        notes: state.notes,
        warnings: state.warnings,
    };
    unmanaged_rows(env, scope, manifest, lock, &state.items, &mut report.drift);
    Ok(report)
}

/// A refusal is a conflict the user must resolve, and any previous, wider
/// rendering comes off disk on the default path — leaving it live would
/// keep exactly the access the refusal exists to prevent. Only what this
/// installation alone holds comes off: the tree a refused tool shares with
/// a tool that still installs stays exactly where it is.
#[allow(clippy::too_many_arguments)]
fn plan_refusals(
    env: &Env,
    scope: &Scope,
    lock: &Lock,
    state: &desired::DesiredState,
    guard: &mut removal::TrashGuard,
    drift: &mut Vec<DriftRow>,
    ops: &mut Vec<PlannedOp>,
    config_edits: &mut config_edits::ConfigEditPlan,
) -> Result<BTreeSet<String>> {
    let refused_keys: BTreeSet<String> = state
        .refused
        .iter()
        .map(|r| crate::lock::entry_key(r.kind, &r.name, r.harness))
        .collect();
    for refusal in &state.refused {
        let key = crate::lock::entry_key(refusal.kind, &refusal.name, refusal.harness);
        let mut removals = Vec::new();
        if let Some(entry) = lock.entries.get(&key) {
            guard.extend(
                &mut removals,
                removal::removal_ops(env, scope, entry, config_edits)?,
            );
        }
        drift.push(DriftRow {
            kind: refusal.kind,
            name: refusal.name.clone(),
            harness: refusal.harness,
            scope: scope.clone(),
            state: DriftState::Conflict,
            detail: match removals.is_empty() {
                false => format!(
                    "{} — the previous installation will be moved to the trash",
                    refusal.reason
                ),
                true => refusal.reason.clone(),
            },
        });
        ops.append(&mut removals);
    }
    Ok(refused_keys)
}

/// Read-only audit for a scope. A legacy or absent manifest still reports
/// unmanaged items; nothing is planned that would touch a legacy file.
pub fn audit(env: &Env, scope: &Scope) -> Result<EngineReport> {
    let scope = &scope.canonical();
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
                warnings: Vec::new(),
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
