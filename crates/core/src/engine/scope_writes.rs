//! Everything a scope plan writes that is not one item's own artifact: the
//! shared config files edits land in, the install record, the manifest's
//! format line, and the settings a project's skills seed.

use crate::apply::{Op, PlannedOp};
use crate::env::Env;
use crate::error::Result;
use crate::lock::{Lock, lock_path};
use crate::manifest::{self, Manifest};
use crate::model::{HarnessId, ItemKind, Scope};

use super::desired::DesiredState;
use super::{DriftRow, DriftState, config_edits};

/// One mutation per config file, whatever asked for it — a single
/// precondition can hold; per-edit preconditions against the same original
/// bytes cannot.
pub(super) fn plan_config_edits(
    config_edits: config_edits::ConfigEditPlan,
    ops: &mut Vec<PlannedOp>,
) -> Result<()> {
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
    Ok(())
}

/// An old-version lock rewrites even when its entries are unchanged — the
/// version bump is itself the change.
pub(super) fn plan_lock_write(
    env: &Env,
    scope: &Scope,
    lock: &Lock,
    new_lock: Lock,
    ops: &mut Vec<PlannedOp>,
) -> Result<()> {
    if new_lock.entries == lock.entries
        && (lock.version == crate::lock::LOCK_VERSION || lock.entries.is_empty())
    {
        return Ok(());
    }
    let path = lock_path(env, scope);
    ops.push(PlannedOp {
        description: "Update the install record".into(),
        op: Op::WriteLock {
            pre: crate::apply::Pre::observed(&path)?,
            path,
            lock: Box::new(new_lock),
        },
    });
    Ok(())
}

/// Upgrade an older-schema manifest through the normal journaled apply.
/// The bump is a surgical text edit — the schema line changes and nothing
/// else does (invariant 10); only if the line has an unexpected spelling
/// does the plan fall back to a full rewrite.
pub(super) fn plan_schema_upgrade(
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

/// Skills may ship `[env]` defaults; missing keys merge into the project's
/// vstack.settings.toml write-if-absent (v1 semantics — a key the user set
/// anywhere in the file is never touched).
pub(super) fn plan_settings_seed(
    scope: &Scope,
    state: &DesiredState,
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
