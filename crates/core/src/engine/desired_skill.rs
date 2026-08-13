use std::path::PathBuf;

use crate::error::Result;
use crate::hash::{hash_files, installation_hash};
use crate::lock::entry_key;
use crate::manifest::Method;
use crate::model::{HarnessId, ItemKind, Scope};
use crate::render::skill::render_skill;

use super::desired::{Artifact, Desired, DesiredState, ItemCtx, native_dir, skill_canonical};

/// One physical skill surface and the harnesses that read it. Codex and Pi
/// both consume `.agents/skills` in a project, so they form one group and
/// carry exactly one variant; every other pairing today reads its own
/// directory. Variants render to the group's combined constraints, and a
/// variant whose bytes match the base tree deduplicates onto it.
struct SurfaceGroup {
    native: PathBuf,
    members: Vec<HarnessId>,
}

/// One rendered variant: the tree's files and their content hash. A group
/// whose cap cannot be honored produces a refused placeholder and installs
/// nothing.
struct Variant {
    files: Vec<(PathBuf, Vec<u8>)>,
    hash: String,
    refused: bool,
}

fn surface_groups(ctx: &ItemCtx) -> Vec<SurfaceGroup> {
    let mut groups: Vec<SurfaceGroup> = Vec::new();
    for harness in &ctx.harnesses {
        let Some(dir) = native_dir(ctx.env, ctx.scope, *harness, ItemKind::Skill) else {
            continue;
        };
        let native = dir.join(ctx.name);
        match groups.iter_mut().find(|group| group.native == native) {
            Some(group) => group.members.push(*harness),
            None => groups.push(SurfaceGroup {
                native,
                members: vec![*harness],
            }),
        }
    }
    groups
}

pub(super) fn desired_skill(ctx: &ItemCtx, state: &mut DesiredState) -> Result<()> {
    let enabled = ctx.decl.enabled;
    let method = ctx.decl.method.unwrap_or(ctx.manifest.install.method);
    if enabled && matches!(ctx.scope, Scope::Project { .. }) {
        let template = ctx.item_path.join(crate::settings_seed::SETTINGS_TEMPLATE);
        if let Some(text) = ctx.sealed.read_if_exists(&template)? {
            for entry in crate::settings_seed::extract_env_entries(&text) {
                if !state.settings_env.iter().any(|e| e.key == entry.key) {
                    state.settings_env.push(entry);
                }
            }
        }
    }
    let groups = surface_groups(ctx);
    if groups.is_empty() {
        return Ok(());
    }
    let mut variants: Vec<Variant> = Vec::new();
    for group in &groups {
        variants.push(render_variant(ctx, state, group, enabled)?);
    }

    // The base tree is the scope's shared location; the group that natively
    // reads it owns it, the first group otherwise. A variant with the base's
    // bytes links to it; a divergent variant lives at its own surface
    // (project) or in the per-tool store (global).
    let base = skill_canonical(ctx.env, ctx.scope, ctx.name);
    let owner = groups
        .iter()
        .position(|group| group.native == base)
        .unwrap_or(0);
    for (index, group) in groups.iter().enumerate() {
        let variant = &variants[index];
        if variant.refused {
            continue;
        }
        let deduped =
            index == owner || (!variants[owner].refused && variant.hash == variants[owner].hash);
        let (canonical, link) = if method == Method::Copy {
            (group.native.clone(), None)
        } else if deduped {
            match group.native == base {
                true => (base.clone(), None),
                false => (base.clone(), Some(group.native.clone())),
            }
        } else {
            match ctx.scope {
                Scope::Project { .. } => (group.native.clone(), None),
                Scope::Global => (
                    ctx.env
                        .rendered_skill_variants_dir(group.members[0].name())
                        .join(ctx.name),
                    Some(group.native.clone()),
                ),
            }
        };
        for harness in &group.members {
            state.items.push(Desired {
                key: entry_key(ItemKind::Skill, ctx.name, *harness),
                kind: ItemKind::Skill,
                name: ctx.name.to_owned(),
                harness: *harness,
                enabled,
                method,
                source_name: ctx.decl.source.clone(),
                provenance: ctx.provenance.to_owned(),
                hash: installation_hash(
                    ctx.sealed,
                    ctx.item_path,
                    ctx.manifest,
                    ItemKind::Skill,
                    ctx.name,
                    *harness,
                )?,
                upstream_skills: None,
                emitted: None,
                artifact: Artifact::Tree {
                    canonical: canonical.clone(),
                    files: variant.files.clone(),
                    link: link.clone(),
                },
            });
        }
    }
    Ok(())
}

/// Render one group's variant under its combined constraints: the tightest
/// byte cap any member enforces, applied after instruction injection.
fn render_variant(
    ctx: &ItemCtx,
    state: &mut DesiredState,
    group: &SurfaceGroup,
    enabled: bool,
) -> Result<Variant> {
    let mut files = render_skill(ctx.sealed, ctx.item_path, ctx.manifest, ctx.name)?;
    let cap = group
        .members
        .iter()
        .filter_map(|h| crate::harness::format_caps(*h).skill_body_max_bytes)
        .min();
    if let Some(cap) = cap {
        let outcome = crate::render::split::enforce_body_cap(files, cap);
        if let Some(reason) = outcome.refusal {
            for harness in &group.members {
                state.refused.push(super::desired::Refused {
                    kind: ItemKind::Skill,
                    name: ctx.name.to_owned(),
                    harness: *harness,
                    reason: reason.clone(),
                });
            }
            return Ok(Variant {
                files: Vec::new(),
                hash: String::new(),
                refused: true,
            });
        }
        let capped_by = group
            .members
            .iter()
            .copied()
            .filter(|h| {
                crate::harness::format_caps(*h)
                    .skill_body_max_bytes
                    .is_some()
            })
            .min_by_key(|h| crate::harness::format_caps(*h).skill_body_max_bytes)
            .unwrap_or(group.members[0]);
        for warning in outcome.warnings {
            state.warnings.push(super::ItemWarning {
                kind: ItemKind::Skill,
                name: ctx.name.to_owned(),
                harness: Some(capped_by),
                message: warning.message,
                remediation: warning.remediation,
            });
        }
        files = outcome.files;
    }
    if !enabled {
        for (rel, _) in &mut files {
            if rel == std::path::Path::new("SKILL.md") {
                *rel = PathBuf::from("SKILL.md.disabled");
            }
        }
    }
    let hash = hash_files(&files);
    Ok(Variant {
        files,
        hash,
        refused: false,
    })
}
