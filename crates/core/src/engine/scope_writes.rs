//! Everything a scope plan writes that is not one item's own artifact: the
//! shared config files edits land in, the install record, the manifest's
//! format line, and the settings a project's skills seed.

use std::collections::BTreeMap;

use crate::apply::{Op, PlannedOp};
use crate::env::Env;
use crate::error::Result;
use crate::lock::{Lock, SourceRev, lock_path};
use crate::manifest::{self, Manifest};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::SourceState;

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

/// Which commit each source resolved to, for the lock to record. What
/// earlier passes resolved is carried forward — a source that is offline
/// today should not lose the commit it was reading yesterday — and a source
/// the manifest no longer declares drops out.
pub(super) fn source_revisions(
    manifest: &Manifest,
    lock: &Lock,
    state: &DesiredState,
) -> BTreeMap<String, SourceRev> {
    let mut revisions: BTreeMap<String, SourceRev> = lock
        .sources
        .iter()
        .filter(|(name, _)| manifest.sources.contains_key(*name))
        .map(|(name, revision)| (name.clone(), revision.clone()))
        .collect();
    for (name, resolution) in &state.sources {
        let SourceState::Ready(ready) = resolution else {
            continue;
        };
        let Some(commit) = ready.commit.clone() else {
            continue;
        };
        revisions.insert(
            name.clone(),
            SourceRev {
                repo: ready.provenance.clone(),
                rev: manifest.sources.get(name).and_then(|decl| decl.rev.clone()),
                commit,
            },
        );
    }
    revisions
}

/// An old-version lock rewrites even when its entries are unchanged — the
/// version bump is itself the change. So does a source that now resolves to
/// another commit, once there are installations to reproduce: with nothing
/// installed there is no record to keep, and no lock file is created for
/// one.
pub(super) fn plan_lock_write(
    env: &Env,
    scope: &Scope,
    lock: &Lock,
    new_lock: Lock,
    ops: &mut Vec<PlannedOp>,
) -> Result<()> {
    if new_lock.entries == lock.entries
        && (new_lock.sources == lock.sources || new_lock.entries.is_empty())
        && new_lock.settings_seeds == lock.settings_seeds
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

/// The schema assignment on one line, rewritten to the new value with every
/// other byte — indentation, spacing style, trailing comment — kept. None
/// when the line is not a plain `schema = <from>` integer assignment; a
/// comment that merely mentions the text must never match.
fn rewrite_schema_line(line: &str, from: u32, to: u32) -> Option<String> {
    let body = line.trim_start();
    let indent = &line[..line.len() - body.len()];
    let body = body.strip_prefix("schema")?;
    let after_key = body.trim_start();
    let key_ws = &body[..body.len() - after_key.len()];
    let after_eq = after_key.strip_prefix('=')?;
    let value = after_eq.trim_start();
    let eq_ws = &after_eq[..after_eq.len() - value.len()];
    let digits = value.len() - value.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let (number, tail) = value.split_at(digits);
    if number != from.to_string() {
        return None;
    }
    if !(tail.trim_start().is_empty() || tail.trim_start().starts_with('#')) {
        return None;
    }
    Some(format!("{indent}schema{key_ws}={eq_ws}{to}{tail}"))
}

/// Upgrade an older-schema manifest through the normal journaled apply.
/// The bump is a surgical text edit — the schema line changes and nothing
/// else does (invariant 10); only if no schema assignment can be found
/// does the plan fall back to a full rewrite.
pub(super) fn plan_schema_upgrade(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    ops: &mut Vec<PlannedOp>,
) -> Result<()> {
    let path = manifest::manifest_path(env, scope);
    let description = "Upgrade vstack.toml to the current format".to_owned();
    let current = crate::fs::read_if_exists(&path)?.unwrap_or_default();
    let mut rewritten = false;
    let upgraded_text: String = current
        .split_inclusive('\n')
        .map(|line| {
            let (body, newline) = match line.strip_suffix('\n') {
                Some(body) => (body, "\n"),
                None => (line, ""),
            };
            match rewritten {
                false => {
                    match rewrite_schema_line(body, manifest.schema, manifest::MANIFEST_SCHEMA) {
                        Some(new_body) => {
                            rewritten = true;
                            format!("{new_body}{newline}")
                        }
                        None => line.to_owned(),
                    }
                }
                true => line.to_owned(),
            }
        })
        .collect();
    let op = match rewritten {
        true => Op::WriteFile {
            pre: crate::apply::Pre::observed(&path)?,
            path,
            bytes: upgraded_text.into_bytes(),
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
/// anywhere in the file is never touched), and seeded comment blocks whose
/// template improved are refreshed while provably unedited — gated by the
/// lock's per-key ledger, which this plan carries forward on `new_lock`.
pub(super) fn plan_settings_seed(
    scope: &Scope,
    state: &DesiredState,
    new_lock: &mut crate::lock::Lock,
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
            cause: None,
        });
        return Ok(());
    }
    let current = crate::fs::read_if_exists(&path)?;
    let (refreshed, updated) = match current.as_deref() {
        Some(text) => crate::settings_seed::refresh_comments(
            text,
            &state.settings_env,
            &mut new_lock.settings_seeds,
        ),
        None => (String::new(), Vec::new()),
    };
    let merged = crate::settings_seed::merge(
        current.as_ref().map(|_| refreshed.as_str()),
        &state.settings_env,
    );
    let (text, added) = match merged {
        Some((text, added)) => (text, added),
        None if !updated.is_empty() => (refreshed, Vec::new()),
        None => return Ok(()),
    };
    crate::settings_seed::record_seeds(&mut new_lock.settings_seeds, &state.settings_env, &added);
    if current.as_deref() == Some(text.as_str()) {
        return Ok(());
    }
    let mut said = Vec::new();
    if !added.is_empty() {
        said.push(format!("seed {}", added.join(", ")));
    }
    if !updated.is_empty() {
        said.push(format!("refresh the comments on {}", updated.join(", ")));
    }
    ops.push(PlannedOp {
        description: format!(
            "Update {} ({})",
            crate::settings_seed::SETTINGS_FILE,
            said.join("; ")
        ),
        op: Op::WriteFile {
            pre: crate::apply::Pre::observed(&path)?,
            path,
            bytes: text.into_bytes(),
        },
    });
    Ok(())
}
