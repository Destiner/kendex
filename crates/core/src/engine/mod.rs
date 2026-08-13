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
mod config_edits;
pub mod desired;
mod desired_agent;
mod desired_kinds;
mod desired_skill;
mod item_plan;
pub mod ops;
mod removal;
mod targets;
mod unmanaged;

use item_plan::plan_item;
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
    };
    let mut written_canonicals: BTreeSet<PathBuf> = BTreeSet::new();
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

    for item in &state.items {
        plan_item(
            item,
            scope,
            lock,
            &mut drift,
            &mut ops,
            &mut config_edits,
            &mut new_lock,
            &mut written_canonicals,
        )?;
    }

    plan_settings_seed(scope, &state, &mut ops, &mut drift)?;

    let refused_keys = plan_refusals(
        env,
        scope,
        lock,
        &state,
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
        &mut drift,
        &mut ops,
        &mut config_edits,
        &mut new_lock,
    )?;

    // One mutation per config file, whatever asked for it — a single
    // precondition can hold; per-edit preconditions against the same
    // original bytes cannot.
    for (path, (labels, edits)) in config_edits.by_file {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        ops.push(PlannedOp {
            description: format!("Update {file} ({})", labels.join(", ")),
            op: Op::EditFile {
                pre: crate::apply::Pre::observed(&path)?,
                path,
                edits,
            },
        });
    }

    // An old-version lock rewrites even when its entries are unchanged —
    // the version bump is itself the change.
    if new_lock.entries != lock.entries
        || (lock.version != crate::lock::LOCK_VERSION && !lock.entries.is_empty())
    {
        let path = lock_path(env, scope);
        ops.push(PlannedOp {
            description: "Update the install record".into(),
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
        warnings: state.warnings,
    };
    unmanaged_rows(env, scope, manifest, lock, &state.items, &mut report.drift);
    Ok(report)
}

/// Upgrade an older-schema manifest through the normal journaled apply.
/// The bump is a surgical text edit — the schema line changes and nothing
/// else does (invariant 10); only if the line has an unexpected spelling
/// does the plan fall back to a full rewrite.
fn plan_schema_upgrade(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    ops: &mut Vec<PlannedOp>,
) -> Result<()> {
    let path = manifest::manifest_path(env, scope);
    let description = "Upgrade vstack.toml to the current format".to_owned();
    let old_line = format!("schema = {}", manifest.schema);
    let new_line = format!("schema = {}", manifest::MANIFEST_SCHEMA);
    let current = crate::fs::read_if_exists(&path)?.unwrap_or_default();
    let op = match current.lines().any(|line| line.trim() == old_line) {
        true => Op::WriteFile {
            pre: crate::apply::Pre::observed(&path)?,
            path,
            bytes: current.replacen(&old_line, &new_line, 1).into_bytes(),
        },
        false => {
            let mut upgraded = manifest.clone();
            upgraded.schema = manifest::MANIFEST_SCHEMA;
            Op::WriteManifest {
                pre: crate::apply::Pre::observed(&path)?,
                path,
                manifest: Box::new(upgraded),
            }
        }
    };
    ops.push(PlannedOp { description, op });
    Ok(())
}

/// A refusal is a conflict the user must resolve, and any previous, wider
/// rendering comes off disk on the default path — leaving it live would
/// keep exactly the access the refusal exists to prevent.
#[allow(clippy::too_many_arguments)]
fn plan_refusals(
    env: &Env,
    scope: &Scope,
    lock: &Lock,
    state: &desired::DesiredState,
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
        let existing = lock.entries.get(&key);
        drift.push(DriftRow {
            kind: refusal.kind,
            name: refusal.name.clone(),
            harness: refusal.harness,
            scope: scope.clone(),
            state: DriftState::Conflict,
            detail: match existing {
                Some(_) => format!(
                    "{} — the previous installation will be moved to the trash",
                    refusal.reason
                ),
                None => refusal.reason.clone(),
            },
        });
        if let Some(entry) = existing {
            ops.extend(removal::removal_ops(env, scope, entry, config_edits)?);
        }
    }
    Ok(refused_keys)
}

/// Skills may ship `[env]` defaults; missing keys merge into the project's
/// vstack.settings.toml write-if-absent (v1 semantics — a key the user set
/// anywhere in the file is never touched).
fn plan_settings_seed(
    scope: &Scope,
    state: &desired::DesiredState,
    ops: &mut Vec<PlannedOp>,
    drift: &mut Vec<DriftRow>,
) -> Result<()> {
    let Scope::Project { root } = scope else {
        return Ok(());
    };
    if state.settings_env.is_empty() {
        return Ok(());
    }
    let path = root.join(crate::settings_seed::SETTINGS_FILE);
    if path.is_symlink() || (path.exists() && !path.is_file()) {
        drift.push(DriftRow {
            kind: ItemKind::Skill,
            name: crate::settings_seed::SETTINGS_FILE.into(),
            harness: HarnessId::Claude,
            scope: scope.clone(),
            state: DriftState::Conflict,
            detail: format!("{} is not a regular file", path.display()),
        });
        return Ok(());
    }
    let current = crate::fs::read_if_exists(&path)?;
    let Some((text, added)) = crate::settings_seed::merge(current.as_deref(), &state.settings_env)
    else {
        return Ok(());
    };
    ops.push(PlannedOp {
        description: format!(
            "Seed {} with {}",
            crate::settings_seed::SETTINGS_FILE,
            added.join(", ")
        ),
        op: Op::WriteFile {
            pre: crate::apply::Pre::observed(&path)?,
            path,
            bytes: text.into_bytes(),
        },
    });
    Ok(())
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
