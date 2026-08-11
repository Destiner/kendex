use std::path::PathBuf;

use super::desired::native_dir;
use super::targets::{HookTarget, disabled_name, hook_target, mcp_registry, plugin_settings};
use crate::apply::{Op, PlannedOp, Pre};
use crate::configedit::ConfigEdit;
use crate::env::Env;
use crate::error::Result;
use crate::hash::hash_tree;
use crate::lock::LockEntry;
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
/// the trash, registrations are reversed by a structured edit. Nothing is
/// planned that would not change the disk, and nothing outside what this
/// entry installed is touched (invariant 6).
pub(super) fn removal_ops(env: &Env, scope: &Scope, entry: &LockEntry) -> Result<Vec<PlannedOp>> {
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
        ops.push(PlannedOp {
            description: format!(
                "Remove {} from {}'s settings",
                entry.name,
                entry.harness.display_name()
            ),
            op: Op::EditFile {
                pre: Pre::HashIs {
                    hash: hash_tree(&path)?,
                },
                path,
                edit,
            },
        });
    }
    Ok(ops)
}
