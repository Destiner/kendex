use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::env::Env;
use crate::error::Result;
use crate::harness::{Surface, adapter};
use crate::hash::{hash_bytes, hash_files, installation_hash};
use crate::lock::{Lock, entry_key};
use crate::manifest::{ItemDecl, Manifest, Method};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::render::skill::render_skill;
use crate::source::{self, SourceState, find_item, source_config};

use super::{desired_agent, desired_kinds};

/// One installation as declaration says it should exist on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct Desired {
    pub key: String,
    pub kind: ItemKind,
    pub name: String,
    pub harness: HarnessId,
    pub enabled: bool,
    pub method: Method,
    pub source_name: String,
    pub provenance: String,
    pub hash: String,
    pub upstream_skills: Option<Vec<String>>,
    pub artifact: Artifact,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Artifact {
    /// A generated file (agents). Disabled installations keep the rendered
    /// content under the `.disabled` name — rename is lossless.
    File { path: PathBuf, bytes: Vec<u8> },
    /// A rendered tree plus the harness-native link to it. `link` is `None`
    /// where the native dir is the canonical location (codex/pi project) or
    /// the method is copy.
    Tree {
        canonical: PathBuf,
        files: Vec<(PathBuf, Vec<u8>)>,
        link: Option<PathBuf>,
    },
    /// An entry inside shared harness config, optionally backed by a script
    /// or instruction file. Each edit is in sync exactly when re-applying it
    /// changes nothing — that idempotency is the drift check, and it is what
    /// keeps every unrelated key in those files intact (invariant 2).
    Registration {
        script: Option<(PathBuf, Vec<u8>)>,
        edits: Vec<(PathBuf, crate::configedit::ConfigEdit)>,
    },
}

#[derive(Debug, Default)]
pub struct DesiredState {
    pub items: Vec<Desired>,
    /// Sources that could not be read (pending remotes, missing paths) and
    /// declared items the source no longer carries.
    pub notes: Vec<String>,
    /// Declarations whose source resolved and whose item was found. What
    /// these produced is the complete truth about them, so a lock entry
    /// they did not produce is stranded, not merely skipped this pass.
    pub processed: BTreeSet<(ItemKind, String)>,
    /// Manifest with upstream skill additions merged in — present only when
    /// the merge changed something and must be written back.
    pub manifest_update: Option<Manifest>,
    /// `[env]` defaults shipped by enabled skills
    /// (vstack.settings.toml.example), first declaration wins per key.
    pub settings_env: Vec<crate::settings_seed::EnvEntry>,
}

impl DesiredState {
    /// A declaration whose source item cannot be parsed. Un-marking it keeps
    /// what it already installed out of the orphan sweep: a source file
    /// someone broke this morning must never uninstall a working artifact.
    pub(super) fn unreadable(&mut self, kind: ItemKind, name: &str, note: String) {
        self.notes.push(note);
        self.processed.remove(&(kind, name.to_owned()));
    }
}

/// The dir a harness natively reads `kind` from at this scope, taken from
/// the same adapter surface declarations the scanner uses.
pub fn native_dir(env: &Env, scope: &Scope, harness: HarnessId, kind: ItemKind) -> Option<PathBuf> {
    let a = adapter(harness);
    let surfaces = match scope {
        Scope::Global => a.global_surfaces(kind, &a.default_global_root(env), env),
        Scope::Project { root } => a.project_surfaces(kind, root, env),
    };
    surfaces.into_iter().find_map(|surface| match surface {
        Surface::FileDir { dir, .. } | Surface::SubdirPerItem { dir, .. } => Some(dir),
        Surface::Structured { .. } => None,
    })
}

fn skill_canonical(env: &Env, scope: &Scope, name: &str) -> PathBuf {
    match scope {
        Scope::Global => env.rendered_skills_dir().join(name),
        Scope::Project { root } => root.join(".agents/skills").join(name),
    }
}

pub(super) fn target_harnesses(
    decl: &ItemDecl,
    manifest: &Manifest,
    kind: ItemKind,
    scope: &Scope,
) -> Vec<HarnessId> {
    let requested = decl
        .harnesses
        .clone()
        .unwrap_or_else(|| manifest.install.harnesses.clone());
    requested
        .into_iter()
        .filter(|harness| {
            let support = crate::harness::capabilities(*harness, kind).install;
            match scope {
                Scope::Global => support.global,
                Scope::Project { .. } => support.project,
            }
        })
        .collect()
}

/// The desired world, computed against the manifest that will be on disk
/// once this plan applies. An upstream skill merge rewrites the manifest,
/// and hashes and renderings must reflect that rewrite — otherwise the very
/// next audit reads the merged manifest and calls a clean install stale. The
/// merge is idempotent, so recomputing against it converges in one repeat.
pub fn desired_state(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    lock: &Lock,
) -> Result<DesiredState> {
    let first = compute(env, scope, manifest, lock)?;
    let Some(merged) = first.manifest_update else {
        return Ok(first);
    };
    let mut second = compute(env, scope, &merged, lock)?;
    second.manifest_update = Some(merged);
    Ok(second)
}

fn compute(env: &Env, scope: &Scope, manifest: &Manifest, lock: &Lock) -> Result<DesiredState> {
    let mut state = DesiredState::default();
    let mut updated_manifest = manifest.clone();
    let mut manifest_changed = false;

    for (kind, table) in [
        (ItemKind::Skill, &manifest.skills),
        (ItemKind::Agent, &manifest.agents),
        (ItemKind::Hook, &manifest.hooks),
        (ItemKind::Command, &manifest.commands),
        (ItemKind::McpServer, &manifest.mcp_servers),
    ] {
        for (name, decl) in table {
            let Some((root, provenance)) =
                resolve_source(env, scope, name, decl, manifest, &mut state.notes)?
            else {
                continue;
            };
            let config = source_config(&root)?;
            let Some(item_path) = find_item(&root, &config, kind, name) else {
                state
                    .notes
                    .push(format!("{name}: not found in source '{}'", decl.source));
                continue;
            };
            state.processed.insert((kind, name.clone()));
            let ctx = ItemCtx {
                env,
                scope,
                manifest,
                lock,
                config: &config,
                root: &root,
                name,
                decl,
                item_path: &item_path,
                provenance: &provenance,
                harnesses: target_harnesses(decl, manifest, kind, scope),
            };
            match kind {
                ItemKind::Skill => desired_skill(&ctx, &mut state)?,
                ItemKind::Agent => desired_agent::desired_agent(
                    &ctx,
                    &mut state,
                    &mut updated_manifest,
                    &mut manifest_changed,
                )?,
                ItemKind::Hook => desired_kinds::desired_hook(&ctx, &mut state)?,
                ItemKind::Command => desired_kinds::desired_command(&ctx, &mut state)?,
                ItemKind::McpServer => desired_kinds::desired_mcp(&ctx, &mut state)?,
                _ => {}
            }
        }
    }
    desired_kinds::desired_plugins(env, scope, manifest, &mut state);

    if manifest_changed {
        state.manifest_update = Some(updated_manifest);
    }
    Ok(state)
}

/// The source root and provenance to build an item from, or `None` with the
/// note that says why this declaration produces nothing this pass.
fn resolve_source(
    env: &Env,
    scope: &Scope,
    name: &str,
    decl: &ItemDecl,
    manifest: &Manifest,
    notes: &mut Vec<String>,
) -> Result<Option<(PathBuf, String)>> {
    match source::resolve(env, scope, &decl.source, manifest)? {
        SourceState::Ready(ready) => Ok(Some((ready.root, ready.provenance))),
        // A disabled source deactivates its installations in place; they stay
        // declared and are not drift.
        SourceState::Disabled { .. } => {
            notes.push(format!(
                "{name}: source '{}' disabled — inactive",
                decl.source
            ));
            Ok(None)
        }
        SourceState::Pending { repo, .. } => {
            notes.push(format!(
                "{name}: source '{}' ({repo}) not fetched yet — skipped",
                decl.source
            ));
            Ok(None)
        }
        SourceState::Missing { path, .. } => {
            notes.push(format!(
                "{name}: source '{}' missing at {} — skipped",
                decl.source,
                path.display()
            ));
            Ok(None)
        }
    }
}

pub(super) struct ItemCtx<'a> {
    pub(super) env: &'a Env,
    pub(super) scope: &'a Scope,
    pub(super) manifest: &'a Manifest,
    pub(super) lock: &'a Lock,
    pub(super) config: &'a crate::source::SourceConfig,
    pub(super) root: &'a std::path::Path,
    pub(super) name: &'a str,
    pub(super) decl: &'a ItemDecl,
    pub(super) item_path: &'a std::path::Path,
    pub(super) provenance: &'a str,
    pub(super) harnesses: Vec<HarnessId>,
}

fn desired_skill(ctx: &ItemCtx, state: &mut DesiredState) -> Result<()> {
    let enabled = ctx.decl.enabled;
    let method = ctx.decl.method.unwrap_or(ctx.manifest.install.method);
    if enabled && matches!(ctx.scope, Scope::Project { .. }) {
        let template = ctx.item_path.join(crate::settings_seed::SETTINGS_TEMPLATE);
        if let Some(text) = crate::fs::read_if_exists(&template)? {
            for entry in crate::settings_seed::extract_env_entries(&text) {
                if !state.settings_env.iter().any(|e| e.key == entry.key) {
                    state.settings_env.push(entry);
                }
            }
        }
    }
    let mut files = render_skill(ctx.item_path, ctx.manifest, ctx.name)?;
    if !enabled {
        for (rel, _) in &mut files {
            if rel == std::path::Path::new("SKILL.md") {
                *rel = PathBuf::from("SKILL.md.disabled");
            }
        }
    }
    for harness in ctx.harnesses.clone() {
        let Some(native) = native_dir(ctx.env, ctx.scope, harness, ItemKind::Skill) else {
            continue;
        };
        let native_item = native.join(ctx.name);
        let canonical = skill_canonical(ctx.env, ctx.scope, ctx.name);
        let (canonical, link) = if method == Method::Copy {
            (native_item, None)
        } else if native_item == canonical {
            (canonical, None)
        } else {
            (canonical, Some(native_item))
        };
        state.items.push(Desired {
            key: entry_key(ItemKind::Skill, ctx.name, harness),
            kind: ItemKind::Skill,
            name: ctx.name.to_owned(),
            harness,
            enabled,
            method,
            source_name: ctx.decl.source.clone(),
            provenance: ctx.provenance.to_owned(),
            hash: installation_hash(
                ctx.item_path,
                ctx.manifest,
                ItemKind::Skill,
                ctx.name,
                harness,
            )?,
            upstream_skills: None,
            artifact: Artifact::Tree {
                canonical,
                files: files.clone(),
                link,
            },
        });
    }
    Ok(())
}

/// Every path a declaration's artifacts occupy, derived from the
/// declaration alone. A source that cannot be read this pass still leaves
/// its installed artifacts on disk — they are ours, and calling them
/// someone else's would invite the user to adopt our own output.
pub(super) fn declared_paths(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    kind: ItemKind,
    name: &str,
    decl: &ItemDecl,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if kind == ItemKind::Skill {
        paths.push(skill_canonical(env, scope, name));
    }
    for harness in target_harnesses(decl, manifest, kind, scope) {
        let Some(native) = native_dir(env, scope, harness, kind) else {
            continue;
        };
        match kind {
            ItemKind::Agent => {
                let base = crate::render::agent::file_name(harness, name);
                paths.push(native.join(format!("{base}.disabled")));
                paths.push(native.join(base));
            }
            _ => paths.push(native.join(name)),
        }
    }
    paths
}

/// Every path an artifact occupies. Cursor keeps hook rules in the same dir
/// as agents and codex shares skill trees with pi: without this, the scanner
/// reports content we just wrote as someone else's.
pub fn artifact_paths(artifact: &Artifact) -> Vec<PathBuf> {
    match artifact {
        Artifact::File { path, .. } => vec![path.clone()],
        Artifact::Tree {
            canonical, link, ..
        } => {
            let mut paths = vec![canonical.clone()];
            paths.extend(link.clone());
            paths
        }
        Artifact::Registration { script, .. } => {
            script.iter().map(|(path, _)| path.clone()).collect()
        }
    }
}

/// The on-disk hash the artifact will have — for clean/dirty comparison.
/// A registration's config edits are compared by re-applying them, not by
/// hash; only its backing file has one.
pub fn artifact_disk_hash(artifact: &Artifact) -> String {
    match artifact {
        Artifact::File { bytes, .. } => hash_bytes(bytes),
        Artifact::Tree { files, .. } => hash_files(files),
        Artifact::Registration { script, .. } => match script {
            Some((_, bytes)) => hash_bytes(bytes),
            None => hash_bytes(&[]),
        },
    }
}
