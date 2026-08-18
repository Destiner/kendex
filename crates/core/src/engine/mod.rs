use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::apply::{Op, Plan, PlannedOp};
use crate::env::Env;
use crate::error::Result;
use crate::lock::{Lock, LockFile, lock_path};
use crate::manifest::{self, Manifest, ManifestFile};
use crate::model::{HarnessId, ItemKind, Scope};

pub mod adopt;
mod adopt_shared;
mod bundles;
mod catalog;
mod config_edits;
mod copilot;
pub mod decisions;
pub mod deps;
pub mod desired;
mod desired_agent;
mod desired_command;
mod desired_kinds;
mod desired_mcp;
mod desired_skill;
mod desired_source;
mod expansion;
pub mod fork;
mod gate;
mod gemini;
mod holds;
mod item_plan;
mod item_source;
mod observed;
pub mod ops;
mod owned;
mod plan_pass;
mod planned;
mod removal;
mod review_hash;
pub mod reviewable;
mod scope_writes;
mod set_change;
mod targets;
mod tree_plan;
mod unmanaged;

pub use gate::{ItemSafety, allow_unsafe_flag};
pub use item_source::{ItemSource, item_source};
pub use observed::{observed_rows, observed_safety};
pub use planned::{PlannedDeclaration, planned_declarations};

/// The conservative "cannot prove these bytes are our render" hold.
pub use removal::edit_holds;

/// Every file path one lock entry put on this machine — what a cheap
/// existence check can stat without reading any source.
pub fn installed_paths(
    env: &crate::env::Env,
    scope: &crate::model::Scope,
    entry: &crate::lock::LockEntry,
) -> Vec<std::path::PathBuf> {
    owned::installed(env, scope, entry).files
}

use desired::desired_state;
use scope_writes::{
    plan_config_edits, plan_lock_write, plan_schema_upgrade, plan_settings_seed, source_revisions,
};
pub use set_change::{KeptInstall, SetChange, SetDirection};
use set_change::{kept_members, set_changes};
use unmanaged::unmanaged_rows;

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

/// Why an installation diverged, when the plan can tell. `LocalEdit` and
/// `Both` are the causes that block writes: the user's bytes are on disk
/// and only an explicit choice may take them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum DriftCause {
    UpstreamChanged,
    LocalEdit,
    Both,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<DriftCause>,
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
    /// What this plan would add to or drop from the installed set.
    pub set_changes: Vec<SetChange>,
    /// Installations this plan leaves alone that nothing needs anymore —
    /// what a removal offers to take with it.
    pub sweepable: Vec<SetChange>,
    /// Members of an uninstalled bundle that stay, and what still accounts
    /// for them — the other half of the preview a bundle removal shows.
    pub kept: Vec<KeptInstall>,
    /// What the safety rules found in the content this plan would write.
    /// Blocked rows also appear as conflicts in `drift`; the rest install
    /// and are worth reading first.
    pub safety: Vec<ItemSafety>,
}

#[derive(Debug, Clone, Default)]
pub struct PlanOptions {
    /// Remove orphaned (locked-but-undeclared) artifacts. Refresh keeps
    /// them (v1 semantics); reconcile and `remove` clean them up.
    pub remove_orphans: bool,
    /// Restrict orphan removal to these names (the `remove` verb).
    pub removal_filter: Option<Vec<String>>,
    /// Also remove installations nothing asked for that nothing needs
    /// anymore — a dependency whose last dependent went away, or one an
    /// upstream item stopped requiring.
    pub sweep_unneeded: bool,
    /// Bundles this plan uninstalls. Their members that survive are named in
    /// the preview with what keeps them, so an uninstall says both halves:
    /// what goes, and what stays.
    pub uninstalled_bundles: Vec<String>,
    /// Items whose safety findings the user has read and accepted. Each one
    /// is recorded in the manifest by the same plan that installs it, bound
    /// to the content, rule set and findings that were reviewed.
    pub allow_unsafe: Vec<String>,
    /// Overwrite installations the user edited by hand. Off, an edited
    /// artifact becomes a conflict and no write touches it; this is the
    /// explicit "discard my edits" everything destructive has to go
    /// through.
    pub overwrite_edited: bool,
    /// Discard edits for these items only, by kind and name — leaving
    /// every other edited item in the scope held. The per-package
    /// "discard" the app offers, which must never take a neighbour's
    /// edits with it, even one that shares a name across kinds.
    pub overwrite_edited_names: Option<Vec<(ItemKind, String)>>,
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
    // Identity first: derived paths and the scope lock key off canonical.
    let scope = &scope.canonical();
    let mut state = desired_state(env, scope, manifest, lock)?;
    // The gate runs before anything is planned for these items: a blocked
    // rendering must never reach the op list, and an override it grants has
    // to ride out on the manifest write this same plan performs.
    let safety = gate::pass(env, scope, manifest, options, &mut state)?;
    let state = state;
    let mut drift = Vec::new();
    let mut ops: Vec<PlannedOp> = Vec::new();
    let mut new_lock = Lock {
        version: crate::lock::LOCK_VERSION,
        entries: BTreeMap::new(),
        sources: source_revisions(manifest, lock, &state),
        // Evidence carried forward; only seeding and refresh may move it.
        settings_seeds: lock.settings_seeds.clone(),
    };
    let mut written = tree_plan::Written::default();
    let mut config_edits = config_edits::ConfigEditPlan::default();

    if let Some(updated) = &state.manifest_update {
        let path = manifest::manifest_path(env, scope);
        let mut updated = updated.clone();
        updated.schema = manifest::MANIFEST_SCHEMA;
        // One write, whatever put it there: skills an agent gained
        // upstream, a review of findings this run was asked to record, or
        // both. Naming only one of them would misdescribe the other.
        let granted = updated.safety_overrides != manifest.safety_overrides;
        ops.push(PlannedOp {
            description: match granted {
                true => "Update vstack.toml with the safety findings you accepted".into(),
                false => "Add new catalog skills to vstack.toml".into(),
            },
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

    plan_pass::plan_items(
        env,
        &state,
        scope,
        lock,
        options,
        &emitted_paths,
        &mut drift,
        &mut ops,
        &mut config_edits,
        &mut new_lock,
        &mut written,
    )?;

    plan_settings_seed(scope, &state, &mut new_lock, &mut ops, &mut drift)?;

    // Trash ops all pass one guard: writes for this pass are already
    // planned, so anything still wanted is known, and no path goes to the
    // trash twice.
    let mut guard = removal::TrashGuard::new(&state.items);
    removal::stale_emitted(&state, lock, &mut guard, &mut ops)?;

    let refused_keys = plan_pass::plan_refusals(
        env,
        scope,
        lock,
        &state,
        &mut guard,
        &mut drift,
        &mut ops,
        &mut config_edits,
    )?;

    let sweepable = removal::orphans(
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
    let set_changes = set_changes(scope, lock, &new_lock);
    let kept = kept_members(scope, lock, &new_lock, &options.uninstalled_bundles);
    plan_lock_write(env, scope, lock, new_lock, &mut ops)?;

    let mut report = EngineReport {
        drift,
        plan: Plan {
            scope: scope.clone(),
            ops,
        },
        notes: state.notes,
        warnings: state.warnings,
        set_changes,
        sweepable,
        kept,
        safety,
    };
    unmanaged_rows(env, scope, manifest, lock, &state.items, &mut report.drift);
    Ok(report)
}

/// Read-only audit for a scope. A legacy or absent manifest still reports
/// unmanaged items; nothing is planned that would touch a legacy file.
pub fn audit(env: &Env, scope: &Scope) -> Result<EngineReport> {
    plan_apply(env, scope, &PlanOptions::default())
}

/// What a refresh would do: regenerate everything declared, and re-derive
/// the closure in both directions — a dependency that appeared upstream is
/// an addition, one that went away leaves an installation nothing needs. The
/// caller previews the set changes before any of it is applied.
pub fn plan_refresh(env: &Env, scope: &Scope) -> Result<EngineReport> {
    plan_apply(
        env,
        scope,
        &PlanOptions {
            sweep_unneeded: true,
            ..PlanOptions::default()
        },
    )
}

/// Plan what disk needs to match declaration, from the manifest as it sits
/// on disk. This is the loader the audit view AND the confirmed apply both
/// use — planning an apply from a mutation-normalized manifest would drop
/// the schema-upgrade op the preview promised, leaving a v0.1 manifest
/// beside a current lock forever.
pub fn plan_apply(env: &Env, scope: &Scope, options: &PlanOptions) -> Result<EngineReport> {
    let scope = &scope.canonical();
    let manifest_file = manifest::load(&manifest::manifest_path(env, scope))?;
    let lock_file = crate::lock::load_file(&lock_path(env, scope))?;
    // Absent reads as an empty current lock — a fresh scope, not a legacy
    // one — so a first-ever install still plans through the normal path.
    let fresh_lock = match &lock_file {
        LockFile::Current(lock) => Some(lock.clone()),
        LockFile::Absent => Some(Lock {
            version: crate::lock::LOCK_VERSION,
            ..Lock::default()
        }),
        LockFile::Legacy { .. } => None,
    };
    if let (ManifestFile::Current(manifest), Some(lock)) = (&manifest_file, &fresh_lock) {
        return plan_scope(env, scope, manifest, lock, options);
    }

    // Either file can't be planned against as-is (a v1 lock, or a v1
    // manifest paired with an already-migrated lock and vice versa) — the
    // scope reads as observation-only rather than failing the whole audit,
    // matching the manifest's existing legacy posture: nothing is planned
    // that would touch a file this build won't write to.
    let mut report = EngineReport {
        drift: Vec::new(),
        plan: Plan {
            scope: scope.clone(),
            ops: Vec::new(),
        },
        notes: Vec::new(),
        warnings: Vec::new(),
        set_changes: Vec::new(),
        sweepable: Vec::new(),
        kept: Vec::new(),
        safety: Vec::new(),
    };
    // One fact, said once: files this build will read but not write. Which
    // of the two is legacy is vstack's problem, not the reader's.
    if matches!(manifest_file, ManifestFile::Legacy { .. })
        || matches!(lock_file, LockFile::Legacy { .. })
    {
        report.notes.push(
            "This scope's vstack files are from version 1 — vstack reads them, but changes nothing here until they are migrated"
                .into(),
        );
    }
    let empty = Manifest::default();
    let lock = fresh_lock.unwrap_or_else(|| Lock {
        version: crate::lock::LOCK_VERSION,
        ..Lock::default()
    });
    unmanaged_rows(env, scope, &empty, &lock, &[], &mut report.drift);
    Ok(report)
}
