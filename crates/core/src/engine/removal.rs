use std::collections::BTreeSet;
use std::path::PathBuf;

use super::desired::{self, Artifact, native_dir};
use super::targets::{HookTarget, disabled_name, hook_target, mcp_registry, plugin_settings};
use super::{DriftRow, DriftState, PlanOptions};
use crate::apply::{Op, PlannedOp, Pre};
use crate::configedit::ConfigEdit;
use crate::env::Env;
use crate::error::Result;
use crate::lock::{Lock, LockEntry};
use crate::manifest::Manifest;
use crate::model::{ItemKind, Scope};
use crate::render::agent::file_name;

/// What one installation put on this machine: files it wrote, and the
/// structured edit that takes its registration back out.
fn installed(env: &Env, scope: &Scope, entry: &LockEntry) -> Owned {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut edits: Vec<(PathBuf, ConfigEdit)> = Vec::new();
    match entry.kind {
        ItemKind::Agent => {
            if let Some(dir) = native_dir(env, scope, entry.harness, ItemKind::Agent) {
                files.push(dir.join(file_name(entry.harness, &entry.name)));
            }
        }
        ItemKind::Skill => {
            if let Some(dir) = native_dir(env, scope, entry.harness, ItemKind::Skill) {
                files.push(dir.join(&entry.name));
            }
            let canonical = match scope {
                Scope::Global => env.rendered_skills_dir().join(&entry.name),
                Scope::Project { root } => root.join(".agents/skills").join(&entry.name),
            };
            if !files.contains(&canonical) {
                files.push(canonical);
            }
        }
        ItemKind::Command => {
            if let Some(dir) = native_dir(env, scope, entry.harness, ItemKind::Command) {
                files.push(dir.join(format!("{}.md", entry.name)));
            }
        }
        ItemKind::Hook => match hook_target(env, scope, entry.harness, &entry.name) {
            Some(HookTarget::Script {
                path,
                command,
                registry,
                ..
            }) => {
                files.push(path);
                // The feature flag codex needed stays on: other hooks may
                // still rely on it, and it enables nothing by itself.
                edits.push((
                    registry,
                    ConfigEdit::RemoveHook {
                        event: None,
                        command,
                    },
                ));
            }
            Some(HookTarget::Instruction {
                path,
                config,
                reference,
            }) => {
                files.push(path);
                edits.push((config, ConfigEdit::OpencodeRemoveInstruction { reference }));
            }
            Some(HookTarget::Rule { path }) => files.push(path),
            None => {}
        },
        ItemKind::McpServer => {
            if let Some(registry) = mcp_registry(env, scope, entry.harness) {
                edits.push((
                    registry,
                    ConfigEdit::RemoveMcpServer {
                        name: entry.name.clone(),
                    },
                ));
            }
        }
        ItemKind::Plugin => {
            if let Some(settings) = plugin_settings(env, scope, entry.harness) {
                edits.push((
                    settings,
                    ConfigEdit::SetPluginEnabled {
                        key: entry.name.clone(),
                        enabled: None,
                    },
                ));
            }
        }
        ItemKind::PiExtension => {}
    }
    Owned { files, edits }
}

struct Owned {
    files: Vec<PathBuf>,
    edits: Vec<(PathBuf, ConfigEdit)>,
}

/// Everything undoing one installation takes: the artifacts we wrote go to
/// the trash, registrations are reversed by a structured edit routed
/// through the per-file collector — a removal and an install editing the
/// same settings file must land in one mutation. Nothing is planned that
/// would not change the disk, and nothing outside what this entry
/// installed is touched (invariant 6).
pub(super) fn removal_ops(
    env: &Env,
    scope: &Scope,
    entry: &LockEntry,
    config_edits: &mut super::config_edits::ConfigEditPlan,
) -> Result<Vec<PlannedOp>> {
    let Owned { files, edits } = installed(env, scope, entry);
    let mut ops = Vec::new();
    for path in files {
        for candidate in [disabled_name(&path), path] {
            if candidate.exists() || candidate.is_symlink() {
                ops.push(PlannedOp {
                    description: format!(
                        "Move {} {}'s files to the trash",
                        entry.kind.name(),
                        entry.name
                    ),
                    op: Op::Trash {
                        path: candidate,
                        pre: Pre::Any,
                    },
                });
            }
        }
    }
    for (path, edit) in edits {
        // An absent config file has nothing of ours in it; creating one to
        // record a removal would be the opposite of removing.
        let Some(current) = crate::fs::read_if_exists(&path)? else {
            continue;
        };
        let updated =
            edit.apply(&current)
                .map_err(|message| crate::error::CoreError::ConfigEdit {
                    path: path.clone(),
                    message,
                })?;
        if updated == current {
            continue;
        }
        config_edits.push(
            path,
            format!("remove {} for {}", entry.name, entry.harness.display_name()),
            edit,
        );
    }
    Ok(ops)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn orphans(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    lock: &Lock,
    state: &desired::DesiredState,
    options: &PlanOptions,
    refused_keys: &BTreeSet<String>,
    drift: &mut Vec<DriftRow>,
    ops: &mut Vec<PlannedOp>,
    config_edits: &mut super::config_edits::ConfigEditPlan,
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
        if desired_keys.contains(key) || refused_keys.contains(key) {
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
                "no longer wanted — will be removed".into()
            } else {
                "left over from an earlier setup; nothing needs it anymore".into()
            },
        });
        if !removable {
            new_lock.entries.insert(key.clone(), entry.clone());
            continue;
        }
        for planned in removal_ops(env, scope, entry, config_edits)? {
            // A skill tree another installation still wants stays put:
            // shared physical targets are reference-counted, not deleted.
            if let Op::Trash { path, .. } = &planned.op
                && (keep_canonical.contains(path.as_path()) || !trashed.insert(path.clone()))
            {
                continue;
            }
            ops.push(planned);
        }
    }
    Ok(())
}
