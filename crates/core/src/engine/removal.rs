use std::collections::BTreeSet;
use std::path::PathBuf;

use super::desired::{self, native_dir};
use super::targets::{
    HookFormat, HookTarget, disabled_name, hook_target, mcp_registry, plugin_settings,
};
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
        // A codex command was written as a skill tree, under a name the
        // collision rules may have changed: the record of what landed
        // beats deriving a path this install never took.
        ItemKind::Command => match &entry.emitted {
            Some(emitted) => files.extend(emitted.paths.iter().cloned()),
            None => {
                if let Some(dir) = native_dir(env, scope, entry.harness, ItemKind::Command) {
                    files.push(dir.join(super::desired_command::command_file(
                        entry.harness,
                        &entry.name,
                    )));
                }
            }
        },
        ItemKind::Hook => match hook_target(env, scope, entry.harness, &entry.name) {
            Some(HookTarget::Script {
                path,
                command,
                registry,
                format,
                ..
            }) => {
                files.push(path);
                // The feature flag codex needed stays on: other hooks may
                // still rely on it, and it enables nothing by itself.
                edits.push((
                    registry,
                    match format {
                        HookFormat::Nested => ConfigEdit::RemoveHook {
                            event: None,
                            command,
                        },
                        HookFormat::Copilot => ConfigEdit::RemoveCopilotHook {
                            event: None,
                            command,
                        },
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
            // Gemini's record of whether a server is on lives in a file of
            // its own and would outlive the declaration it describes. That
            // file is one for the whole machine, so only a global-scope
            // removal takes an entry out of it: a project holds the project
            // lock, and clearing the record there would switch a server on
            // everywhere for a removal that was never meant to leave.
            if entry.harness == crate::model::HarnessId::Gemini && matches!(scope, Scope::Global) {
                edits.push((
                    crate::harness::gemini::settings::mcp_enablement_file(env),
                    ConfigEdit::SetGeminiMcpEnabled {
                        name: entry.name.clone(),
                        enabled: None,
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

/// Which paths a Trash op may still take. Several lock entries name one
/// physical tree — codex and pi read the same skill directory — so a removal
/// must not move a tree another installation still wants, and must not move
/// the same tree twice: the second op finds nothing there and fails the
/// whole apply.
pub(super) struct TrashGuard {
    keep: BTreeSet<PathBuf>,
    trashed: BTreeSet<PathBuf>,
}

impl TrashGuard {
    pub(super) fn new(items: &[desired::Desired]) -> TrashGuard {
        let keep = items
            .iter()
            .flat_map(|item| desired::artifact_paths(&item.artifact))
            .collect();
        TrashGuard {
            keep,
            trashed: BTreeSet::new(),
        }
    }

    fn allows(&mut self, op: &Op) -> bool {
        let Op::Trash { path, .. } = op else {
            return true;
        };
        !self.keep.contains(path) && self.trashed.insert(path.clone())
    }

    pub(super) fn extend(
        &mut self,
        ops: &mut Vec<PlannedOp>,
        planned: impl IntoIterator<Item = PlannedOp>,
    ) {
        ops.extend(planned.into_iter().filter(|p| self.allows(&p.op)));
    }
}

/// An earlier install of a still-declared item wrote somewhere this one will
/// not: a codex command whose emitted name changed when a skill claimed it.
/// The tree it left is ours and nobody wants it now — without this it stays
/// on disk forever, offered by the tool under a name nobody declared.
pub(super) fn stale_emitted(
    state: &desired::DesiredState,
    lock: &Lock,
    guard: &mut TrashGuard,
    ops: &mut Vec<PlannedOp>,
) {
    for item in &state.items {
        let Some(previous) = lock
            .entries
            .get(&item.key)
            .and_then(|entry| entry.emitted.as_ref())
        else {
            continue;
        };
        let current = item.emitted.iter().flat_map(|e| e.paths.iter());
        for path in &previous.paths {
            if current.clone().any(|kept| kept == path) {
                continue;
            }
            if !path.exists() && !path.is_symlink() {
                continue;
            }
            let planned = PlannedOp {
                description: format!(
                    "Move {} {}'s old files to the trash",
                    item.kind.name(),
                    item.name
                ),
                op: Op::Trash {
                    path: path.clone(),
                    pre: Pre::Any,
                },
            };
            guard.extend(ops, [planned]);
        }
    }
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
    guard: &mut TrashGuard,
    drift: &mut Vec<DriftRow>,
    ops: &mut Vec<PlannedOp>,
    config_edits: &mut super::config_edits::ConfigEditPlan,
    new_lock: &mut Lock,
) -> Result<()> {
    let desired_keys: BTreeSet<&String> = state.items.iter().map(|d| &d.key).collect();

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
        guard.extend(ops, removal_ops(env, scope, entry, config_edits)?);
    }
    Ok(())
}
