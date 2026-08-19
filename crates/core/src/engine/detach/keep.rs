//! Keeping a marketplace's packages: copying each installation's source-form
//! bytes into the scope's local source so the package stays after the source is
//! gone. The declaration flip and source removal live in the parent module;
//! this file owns the byte copy and its local-target preflight.

use std::path::PathBuf;

use crate::apply::{Op, Plan, PlannedOp, Pre};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::{ForkProvenance, ItemDecl, LOCAL_SOURCE_NAME, Manifest};
use crate::model::{ItemKind, Scope};
use crate::source::local_source_root;

use super::{ClosureItem, closure, edited_items};

/// The path in the local source one detached item's source-form bytes are
/// written to. A `plugin/item` name nests one directory level, the shape the
/// local reader lists back.
fn local_target(local_root: &std::path::Path, kind: ItemKind, name: &str) -> Result<PathBuf> {
    Ok(match kind {
        ItemKind::Skill => local_root.join("skills").join(name),
        ItemKind::Agent => local_root.join("agents").join(format!("{name}.md")),
        ItemKind::Hook => local_root.join("hooks").join(format!("{name}.sh")),
        ItemKind::Command => local_root.join("commands").join(format!("{name}.md")),
        ItemKind::McpServer => local_root.join("mcp").join(format!("{name}.toml")),
        other => {
            return Err(CoreError::ItemNotInSource {
                name: name.to_owned(),
                source_name: format!("detach does not support {} yet", other.name()),
            });
        }
    })
}

/// Unsubscribe but keep the packages: convert each installation to a local one
/// and remove the source. This copies each item's **source-form** bytes from
/// the catalog at the exact commit it was installed from into the scope's local
/// source, flips its declaration to `local`, and records the conversion as a
/// fork whose bytes did not change. The local writes are ordered before the
/// declaration flip in one plan, so a failure mid-apply rolls the whole
/// conversion back (invariant 11).
///
/// An installation the user has edited is not converted from source form — that
/// would silently drop the edit — so detach refuses while any package is
/// edited, naming them: fork or discard each first. (Routing an edited package
/// through fork capture inside the same plan is the remaining half.)
pub fn source(env: &Env, scope: &Scope, source_name: &str) -> Result<Plan> {
    let scope = scope.canonical();
    let mut manifest = crate::engine::ops::manifest_for_mutation(env, &scope)?;
    let closure = closure(env, &scope, source_name, &manifest)?;
    let lock = crate::lock::load(&crate::lock::lock_path(env, &scope))?;

    // An edited installation cannot be recovered from source form; name every
    // one and refuse rather than lose the edit.
    let edited = edited_items(env, &scope, &closure, &lock);
    if !edited.is_empty() {
        return Err(CoreError::DetachEdited { names: edited });
    }

    let local_root = local_source_root(env, &scope);
    let mut ops = Vec::new();
    for item in &closure.items {
        // Read this item's source-form bytes at the exact commit it installed
        // from — not the source head, which may have moved.
        let commit = lock
            .entries
            .values()
            .find(|e| e.kind == item.kind && e.name == item.name)
            .and_then(|e| e.source_commit.clone());
        let files = source_form(env, &scope, &manifest, item, commit.as_deref())?;
        let target = local_target(&local_root, item.kind, &item.name)?;
        ops.extend(capture_to_local(item.kind, &item.name, &target, files)?);
    }

    // Flip every converted item to the local source and record the fork.
    for item in &closure.items {
        let provenance = ForkProvenance {
            repo: manifest
                .sources
                .get(&item.decl.source)
                .and_then(|s| s.repo.clone()),
            source: item.decl.source.clone(),
            commit: lock
                .entries
                .values()
                .find(|e| e.kind == item.kind && e.name == item.name)
                .and_then(|e| e.source_commit.clone()),
            forked_at: crate::clock::timestamp(),
        };
        // A derived member or dependency becomes a plain declaration; a
        // declared item flips in place. Either way it now reads `local`, holds
        // nothing, and its bundle/dependency membership is a request of its own.
        let decl = manifest
            .declared_mut(item.kind)
            .entry(item.name.clone())
            .or_insert_with(|| ItemDecl::from_source(LOCAL_SOURCE_NAME));
        decl.source = LOCAL_SOURCE_NAME.to_owned();
        decl.rev = None;
        manifest
            .forks
            .entry(item.kind)
            .or_default()
            .insert(item.name.clone(), provenance);
    }
    manifest
        .bundles
        .retain(|_, decl| decl.source != source_name);
    manifest.sources.remove(source_name);

    let manifest_path = crate::manifest::manifest_path(env, &scope);
    ops.push(PlannedOp {
        description: format!("keep {source_name}'s packages as your own in kendex.toml"),
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

/// One item's source-form files, read through the sealed catalog at the commit
/// it was installed from: the skill's tree, or the single file the other kinds
/// keep. `(relative path, bytes)`, ready to write under the local target.
fn source_form(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    item: &ClosureItem,
    commit: Option<&str>,
) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let resolved = match crate::source::resolve_at(env, scope, &item.decl.source, manifest, commit)?
    {
        crate::source::SourceState::Ready(ready) => ready,
        _ => {
            return Err(CoreError::SourcePending {
                name: item.decl.source.clone(),
            });
        }
    };
    let sealed = crate::source_read::SealedSource::open(&resolved.root)?;
    let config = crate::source::source_config_for(&sealed, &resolved.provenance)?;
    let Some(path) = crate::source::find_item(&sealed, &config, item.kind, &item.name) else {
        return Err(CoreError::ItemNotInSource {
            name: item.name.clone(),
            source_name: item.decl.source.clone(),
        });
    };
    match item.kind {
        ItemKind::Skill => sealed.collect_skill_tree(&path),
        _ => {
            let bytes = sealed.read(&path)?;
            let file = path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&item.name));
            Ok(vec![(file, bytes)])
        }
    }
}

/// The write op for one detached item, after preflighting the local target:
/// an occupied target holding different bytes (an earlier adopt, fork, or
/// detach of the same kind and name) is a refusal naming it — detach never
/// overwrites what is already local (invariants 4 and 6). A target already
/// holding the same bytes needs no write.
fn capture_to_local(
    kind: ItemKind,
    name: &str,
    target: &std::path::Path,
    files: Vec<(PathBuf, Vec<u8>)>,
) -> Result<Vec<PlannedOp>> {
    if target.exists() {
        let existing = crate::hash::hash_tree(target)?;
        let incoming = match kind {
            ItemKind::Skill => crate::hash::hash_files(&files),
            _ => crate::hash::hash_bytes(&files[0].1),
        };
        if existing == incoming {
            return Ok(Vec::new());
        }
        return Err(CoreError::LocalTargetOccupied {
            kind,
            name: name.to_owned(),
            path: target.to_path_buf(),
        });
    }
    let op = match kind {
        ItemKind::Skill => Op::WriteTree {
            root: target.to_path_buf(),
            files,
            pre: Pre::Absent,
        },
        _ => Op::WriteFile {
            path: target.to_path_buf(),
            bytes: files.into_iter().next().map(|(_, b)| b).unwrap_or_default(),
            pre: Pre::Absent,
        },
    };
    Ok(vec![PlannedOp {
        description: format!("keep {} {name} in your local source", kind.name()),
        op,
    }])
}
