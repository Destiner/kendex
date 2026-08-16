//! Keeping a user's edits: the edited installation becomes a local package
//! under the same name. Fork is adopt with provenance — the bytes move into
//! the scope's local source, the declaration flips to `local`, and the
//! manifest records what it replaced. The name never changes, so nothing
//! that depends on it breaks.

use std::fs;
use std::path::PathBuf;

use super::desired::{native_dir, skill_canonical};
use super::ops::manifest_for_mutation;
use crate::apply::{Op, Plan, PlannedOp, Pre};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::{self, ForkProvenance, LOCAL_SOURCE_NAME};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::local_source_root;

/// Turn one edited installation into a local fork. The harness names which
/// installation's bytes are captured — an agent renders per tool, and the
/// edit lives in exactly one rendering. Skills capture the canonical tree,
/// the one place every tool's link resolves to.
///
/// The plan: capture the edited bytes into the local source (an earlier
/// local copy goes to the trash first, never overwritten), trash the edited
/// artifact so the follow-up apply re-renders it from the fork, and write
/// the manifest — source flipped to `local`, any hold cleared (a fork of a
/// local directory has no revisions), provenance recorded.
pub fn fork(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    harness: HarnessId,
) -> Result<Plan> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let Some(decl) = manifest.declared(kind).get(name).cloned() else {
        return Err(CoreError::NotDeclared {
            kind,
            name: name.to_owned(),
        });
    };
    let edited = match kind {
        ItemKind::Skill => skill_canonical(env, scope, name),
        ItemKind::Agent => {
            let Some(dir) = native_dir(env, scope, harness, ItemKind::Agent) else {
                return Err(CoreError::ItemNotFound {
                    kind,
                    name: name.to_owned(),
                    harness,
                });
            };
            existing_or_disabled(dir.join(crate::render::agent::file_name(harness, name)))
        }
        other => {
            return Err(CoreError::ItemNotInSource {
                name: name.to_owned(),
                source_name: format!("fork does not support {} yet", other.name()),
            });
        }
    };
    if edited.is_symlink() || !edited.exists() {
        return Err(CoreError::ItemNotFound {
            kind,
            name: name.to_owned(),
            harness,
        });
    }

    let mut ops = capture_ops(env, scope, kind, name, &edited)?;

    let provenance = ForkProvenance {
        repo: manifest
            .sources
            .get(&decl.source)
            .and_then(|s| s.repo.clone()),
        source: decl.source.clone(),
        commit: crate::lock::load(&crate::lock::lock_path(env, scope))?
            .entries
            .values()
            .filter(|entry| entry.kind == kind && entry.name == name)
            .find_map(|entry| entry.source_commit.clone()),
        forked_at: crate::clock::timestamp(),
    };
    let entry = manifest
        .declared_mut(kind)
        .get_mut(name)
        .unwrap_or_else(|| unreachable!("declared above"));
    entry.source = LOCAL_SOURCE_NAME.to_owned();
    entry.rev = None;
    manifest
        .forks
        .entry(kind)
        .or_default()
        .insert(name.to_owned(), provenance);

    let manifest_path = manifest::manifest_path(env, scope);
    ops.push(PlannedOp {
        description: format!("record the fork of {name} in vstack.toml"),
        op: Op::WriteManifest {
            pre: Pre::observed(&manifest_path)?,
            path: manifest_path,
            manifest: Box::new(manifest),
        },
    });
    Ok(Plan {
        scope: scope.clone(),
        ops,
    })
}

/// The ops that move the edited bytes into the local source: an earlier
/// local copy goes to the trash (never overwritten in place), the bytes
/// are captured under the same name, and the edited artifact itself goes
/// to the trash — bound to the exact bytes just captured (invariant 7) —
/// so the follow-up apply renders the fork in its place.
fn capture_ops(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    edited: &std::path::Path,
) -> Result<Vec<PlannedOp>> {
    let local_root = local_source_root(env, scope);
    let local_item = match kind {
        ItemKind::Skill => local_root.join("skills").join(name),
        _ => local_root.join("agents").join(format!("{name}.md")),
    };
    let mut ops = Vec::new();
    if local_item.exists() {
        ops.push(PlannedOp {
            description: format!("trash the local source's earlier copy of {name}"),
            op: Op::Trash {
                path: local_item.clone(),
                pre: Pre::HashIs {
                    hash: crate::hash::hash_tree(&local_item)?,
                },
            },
        });
    }
    let capture = match kind {
        ItemKind::Skill => Op::WriteTree {
            root: local_item,
            files: super::adopt::read_tree(edited)?,
            pre: Pre::Absent,
        },
        _ => Op::WriteFile {
            path: local_item,
            bytes: fs::read(edited).map_err(|e| CoreError::io(edited, e))?,
            pre: Pre::Absent,
        },
    };
    ops.push(PlannedOp {
        description: format!("keep the edited {} {name} as a local fork", kind.name()),
        op: capture,
    });
    ops.push(PlannedOp {
        description: format!("clear the edited install of {name} for re-render"),
        op: Op::Trash {
            pre: Pre::HashIs {
                hash: crate::hash::hash_tree(edited)?,
            },
            path: edited.to_path_buf(),
        },
    });
    Ok(ops)
}

/// Rename a fork. Only a fork nothing depends on may change its installed
/// name: dependents and bundles refer to the old one, and a rename that
/// breaks them is not a rename, it is a removal wearing one's clothes.
pub fn rename_fork(env: &Env, scope: &Scope, kind: ItemKind, old: &str, new: &str) -> Result<Plan> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    if !manifest
        .forks
        .get(&kind)
        .is_some_and(|forks| forks.contains_key(old))
    {
        return Err(CoreError::NotDeclared {
            kind,
            name: old.to_owned(),
        });
    }
    if let Some(problem) = crate::names::item_problem(new) {
        return Err(CoreError::ItemNotInSource {
            name: problem,
            source_name: "the new name".to_owned(),
        });
    }
    if manifest.declared(kind).contains_key(new) {
        return Err(CoreError::SourceCollision {
            name: new.to_owned(),
            existing: "this scope's manifest".to_owned(),
            requested: LOCAL_SOURCE_NAME.to_owned(),
        });
    }
    let lock = crate::lock::load(&crate::lock::lock_path(env, scope))?;
    let depended_on = lock
        .entries
        .values()
        .filter(|entry| entry.kind == kind && entry.name == old)
        .flat_map(|entry| entry.reasons.iter())
        .any(|reason| !matches!(reason, crate::lock::Reason::Requested));
    if depended_on {
        return Err(CoreError::ManifestInvalid {
            path: manifest::manifest_path(env, scope),
            findings: vec![format!(
                "{}.{old}: other items depend on this name — fix: rename what depends on it first, or keep the name",
                kind.name()
            )],
        });
    }

    let local_root = local_source_root(env, scope);
    let (from, to) = match kind {
        ItemKind::Skill => (
            local_root.join("skills").join(old),
            local_root.join("skills").join(new),
        ),
        _ => (
            local_root.join("agents").join(format!("{old}.md")),
            local_root.join("agents").join(format!("{new}.md")),
        ),
    };
    let mut ops = Vec::new();
    if from.exists() {
        ops.push(PlannedOp {
            description: format!("rename the fork's files to {new}"),
            op: Op::Rename {
                from,
                to,
                to_pre: Pre::Absent,
            },
        });
    }
    let Some(decl) = manifest.declared_mut(kind).remove(old) else {
        return Err(CoreError::NotDeclared {
            kind,
            name: old.to_owned(),
        });
    };
    manifest.declared_mut(kind).insert(new.to_owned(), decl);
    if let Some(forks) = manifest.forks.get_mut(&kind)
        && let Some(provenance) = forks.remove(old)
    {
        forks.insert(new.to_owned(), provenance);
    }
    let manifest_path = manifest::manifest_path(env, scope);
    ops.push(PlannedOp {
        description: format!("record the rename to {new} in vstack.toml"),
        op: Op::WriteManifest {
            pre: Pre::observed(&manifest_path)?,
            path: manifest_path,
            manifest: Box::new(manifest),
        },
    });
    Ok(Plan {
        scope: scope.clone(),
        ops,
    })
}

/// A disabled installation keeps its bytes under the `.disabled` name.
fn existing_or_disabled(path: PathBuf) -> PathBuf {
    if path.exists() || path.is_symlink() {
        return path;
    }
    let disabled = PathBuf::from(format!("{}.disabled", path.display()));
    if disabled.exists() { disabled } else { path }
}
