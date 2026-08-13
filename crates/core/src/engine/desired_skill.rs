use std::path::PathBuf;

use crate::error::Result;
use crate::hash::installation_hash;
use crate::lock::entry_key;
use crate::manifest::Method;
use crate::model::{ItemKind, Scope};
use crate::render::skill::render_skill;

use super::desired::{Artifact, Desired, DesiredState, ItemCtx, native_dir, skill_canonical};

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
    let mut files = render_skill(ctx.sealed, ctx.item_path, ctx.manifest, ctx.name)?;
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
                ctx.sealed,
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
