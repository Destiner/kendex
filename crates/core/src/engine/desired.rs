use std::path::PathBuf;

use crate::env::Env;
use crate::error::Result;
use crate::harness::{Surface, adapter};
use crate::hash::{hash_bytes, hash_files, installation_hash};
use crate::lock::{Lock, entry_key};
use crate::manifest::{ItemDecl, Manifest, Method};
use crate::mapping::effective_skills;
use crate::model::{HarnessId, ItemKind, Scope};
use crate::render::agent::{
    EffectiveAgent, file_name, generate, hooks_for_agent, merge_overrides, merged_instructions,
    parse_source_agent,
};
use crate::render::skill::render_skill;
use crate::source::{self, SourceState, find_item, list_items, source_config};

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
}

#[derive(Debug, Default)]
pub struct DesiredState {
    pub items: Vec<Desired>,
    /// Sources that could not be read (pending remotes, missing paths) and
    /// declared items the source no longer carries.
    pub notes: Vec<String>,
    /// Manifest with upstream skill additions merged in — present only when
    /// the merge changed something and must be written back.
    pub manifest_update: Option<Manifest>,
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

fn target_harnesses(
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

pub fn desired_state(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    lock: &Lock,
) -> Result<DesiredState> {
    let mut state = DesiredState::default();
    let mut updated_manifest = manifest.clone();
    let mut manifest_changed = false;

    for (kind, table) in [
        (ItemKind::Skill, &manifest.skills),
        (ItemKind::Agent, &manifest.agents),
    ] {
        for (name, decl) in table {
            let source_state = source::resolve(env, scope, &decl.source, manifest)?;
            let (root, provenance) = match &source_state {
                SourceState::Ready(ready) => (ready.root.clone(), ready.provenance.clone()),
                SourceState::Disabled { .. } => {
                    // A disabled source deactivates its installations in
                    // place; they stay declared and are not drift.
                    state.notes.push(format!(
                        "{name}: source '{}' disabled — inactive",
                        decl.source
                    ));
                    continue;
                }
                SourceState::Pending { repo, .. } => {
                    state.notes.push(format!(
                        "{name}: source '{}' ({repo}) not fetched yet — skipped",
                        decl.source
                    ));
                    continue;
                }
                SourceState::Missing { path, .. } => {
                    state.notes.push(format!(
                        "{name}: source '{}' missing at {} — skipped",
                        decl.source,
                        path.display()
                    ));
                    continue;
                }
            };
            let config = source_config(&root)?;
            let Some(item_path) = find_item(&root, &config, kind, name) else {
                state
                    .notes
                    .push(format!("{name}: not found in source '{}'", decl.source));
                continue;
            };
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
                ItemKind::Agent => desired_agent(
                    &ctx,
                    &mut state,
                    &mut updated_manifest,
                    &mut manifest_changed,
                )?,
                _ => {}
            }
        }
    }

    if manifest_changed {
        state.manifest_update = Some(updated_manifest);
    }
    Ok(state)
}

struct ItemCtx<'a> {
    env: &'a Env,
    scope: &'a Scope,
    manifest: &'a Manifest,
    lock: &'a Lock,
    config: &'a crate::source::SourceConfig,
    root: &'a std::path::Path,
    name: &'a str,
    decl: &'a ItemDecl,
    item_path: &'a std::path::Path,
    provenance: &'a str,
    harnesses: Vec<HarnessId>,
}

fn desired_skill(ctx: &ItemCtx, state: &mut DesiredState) -> Result<()> {
    let enabled = ctx.decl.enabled;
    let method = ctx.decl.method.unwrap_or(ctx.manifest.install.method);
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

fn desired_agent(
    ctx: &ItemCtx,
    state: &mut DesiredState,
    updated_manifest: &mut Manifest,
    manifest_changed: &mut bool,
) -> Result<()> {
    let enabled = ctx.decl.enabled;
    let text = std::fs::read_to_string(ctx.item_path)
        .map_err(|e| crate::error::CoreError::io(ctx.item_path, e))?;
    let source_agent = match parse_source_agent(&text) {
        Ok(agent) => agent,
        Err(problem) => {
            state
                .notes
                .push(format!("{}: unreadable agent — {problem}", ctx.name));
            return Ok(());
        }
    };
    let available = list_items(ctx.root, ctx.config, ItemKind::Skill);
    let recorded = ctx.harnesses.iter().find_map(|h| {
        ctx.lock
            .entries
            .get(&entry_key(ItemKind::Agent, ctx.name, *h))
            .and_then(|entry| entry.upstream_skills.clone())
    });
    let skills = effective_skills(
        ctx.name,
        source_agent.role,
        ctx.manifest,
        ctx.config,
        &available,
        recorded.as_deref(),
    );
    if !skills.manifest_additions.is_empty() {
        let entry = updated_manifest
            .agent_skills
            .entry(ctx.name.to_owned())
            .or_default();
        for skill in &skills.manifest_additions {
            if !entry.contains(skill) {
                entry.push(skill.clone());
            }
        }
        *manifest_changed = true;
    }
    for harness in ctx.harnesses.clone() {
        let Some(native) = native_dir(ctx.env, ctx.scope, harness, ItemKind::Agent) else {
            continue;
        };
        let overrides = merge_overrides(
            ctx.config
                .frontmatter
                .get(harness.name())
                .and_then(|by_agent| by_agent.get(ctx.name)),
            ctx.manifest
                .agent_frontmatter
                .get(harness.name())
                .and_then(|by_agent| by_agent.get(ctx.name)),
        );
        let effective = EffectiveAgent {
            source: &source_agent,
            harness,
            scope: ctx.scope,
            skills: skills.effective.clone(),
            overrides,
            launch_instructions: merged_instructions(
                &ctx.manifest.agent_launch_instructions,
                ctx.name,
            ),
            additional_instructions: merged_instructions(
                &ctx.manifest.agent_additional_instructions,
                ctx.name,
            ),
            custom_hooks: hooks_for_agent(ctx.manifest, &source_agent),
        };
        let rendered = generate(&effective);
        let base = file_name(harness, ctx.name);
        let file = if enabled {
            native.join(&base)
        } else {
            native.join(format!("{base}.disabled"))
        };
        state.items.push(Desired {
            key: entry_key(ItemKind::Agent, ctx.name, harness),
            kind: ItemKind::Agent,
            name: ctx.name.to_owned(),
            harness,
            enabled,
            method: Method::Copy,
            source_name: ctx.decl.source.clone(),
            provenance: ctx.provenance.to_owned(),
            hash: installation_hash(
                ctx.item_path,
                ctx.manifest,
                ItemKind::Agent,
                ctx.name,
                harness,
            )?,
            upstream_skills: Some(skills.upstream_now.clone()),
            artifact: Artifact::File {
                path: file,
                bytes: rendered.into_bytes(),
            },
        });
    }
    Ok(())
}

/// The on-disk hash the artifact will have — for clean/dirty comparison.
pub fn artifact_disk_hash(artifact: &Artifact) -> String {
    match artifact {
        Artifact::File { bytes, .. } => hash_bytes(bytes),
        Artifact::Tree { files, .. } => hash_files(files),
    }
}
