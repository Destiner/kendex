use crate::error::Result;
use crate::hash::installation_hash;
use crate::lock::entry_key;
use crate::manifest::{Manifest, Method};
use crate::mapping::{EffectiveSkills, effective_skills};
use crate::model::ItemKind;
use crate::render::agent::{
    EffectiveAgent, Role, file_name, generate, hooks_for_agent, merge_overrides,
    merged_instructions, parse_source_agent,
};
use crate::render::permission::PermissionIntent;
use crate::source::list_items;

use super::desired::{Artifact, Desired, DesiredState, ItemCtx, native_dir};

/// The agent's skill list, merging anything upstream added since the last
/// sync back into the manifest so the declaration keeps saying what the
/// agent actually renders with.
fn assigned_skills(
    ctx: &ItemCtx,
    role: Option<Role>,
    updated_manifest: &mut Manifest,
    manifest_changed: &mut bool,
) -> EffectiveSkills {
    let available = list_items(ctx.sealed, ctx.config, ItemKind::Skill);
    let recorded = ctx.harnesses.iter().find_map(|h| {
        ctx.lock
            .entries
            .get(&entry_key(ItemKind::Agent, ctx.name, *h))
            .and_then(|entry| entry.upstream_skills.clone())
    });
    let skills = effective_skills(
        ctx.name,
        role,
        ctx.manifest,
        ctx.config,
        &available,
        recorded.as_deref(),
    );
    if skills.manifest_additions.is_empty() {
        return skills;
    }
    let entry = updated_manifest
        .agent_skills
        .entry(merge_key(ctx.manifest, ctx.name))
        .or_default();
    for skill in &skills.manifest_additions {
        if !entry.contains(skill) {
            entry.push(skill.clone());
        }
    }
    *manifest_changed = true;
    skills
}

/// The `agent_skills` key the effective list was read from. Writing
/// additions anywhere else creates an entry that shadows the one being read,
/// and the shadowed skills silently vanish from the next rendering.
fn merge_key(manifest: &Manifest, name: &str) -> String {
    if manifest.agent_skills.contains_key(name) {
        return name.to_owned();
    }
    let stripped = crate::mapping::skill_match_prefix(name);
    match manifest.agent_skills.contains_key(stripped) {
        true => stripped.to_owned(),
        false => name.to_owned(),
    }
}

/// Source-catalog defaults merged with project overrides for one harness,
/// and the permission intent that merge produces.
fn harness_overrides(
    ctx: &ItemCtx,
    source_agent: &crate::render::agent::SourceAgent,
    harness: crate::model::HarnessId,
) -> (crate::manifest::FrontmatterOverrides, PermissionIntent) {
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
    let permissions = PermissionIntent::effective(
        &source_agent.permissions,
        overrides.allow_tools.as_deref(),
        overrides.deny_tools.as_deref(),
    );
    (overrides, permissions)
}

/// Agents are generated, never linked: every harness gets its own rendering
/// of the same source agent, overwritten on each apply.
pub(super) fn desired_agent(
    ctx: &ItemCtx,
    state: &mut DesiredState,
    updated_manifest: &mut Manifest,
    manifest_changed: &mut bool,
) -> Result<()> {
    let enabled = ctx.decl.enabled;
    let text = ctx.sealed.read_to_string(ctx.item_path)?;
    let source_agent = match parse_source_agent(&text) {
        Ok(agent) => agent,
        Err(problem) => {
            state.unreadable(
                ItemKind::Agent,
                ctx.name,
                format!("{}: unreadable agent — {problem}", ctx.name),
            );
            return Ok(());
        }
    };
    for warning in &source_agent.warnings {
        state.notes.push(format!("{}: {warning}", ctx.name));
    }
    let skills = assigned_skills(ctx, source_agent.role, updated_manifest, manifest_changed);
    for harness in ctx.harnesses.clone() {
        let Some(native) = native_dir(ctx.env, ctx.scope, harness, ItemKind::Agent) else {
            continue;
        };
        let (overrides, permissions) = harness_overrides(ctx, &source_agent, harness);
        let effective = EffectiveAgent {
            source: &source_agent,
            harness,
            scope: ctx.scope,
            skills: skills.effective.clone(),
            overrides,
            permissions,
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
        let rendered = match generate(&effective) {
            Ok(rendered) => rendered,
            // A refusal produces no artifact for this harness; the plan
            // turns it into a conflict row plus removal of any previous,
            // wider rendering — never a silent widen, never a leftover.
            Err(refusal) => {
                state.refused.push(super::desired::Refused {
                    kind: ItemKind::Agent,
                    name: ctx.name.to_owned(),
                    harness,
                    reason: refusal,
                });
                continue;
            }
        };
        for warning in &rendered.warnings {
            state.notes.push(format!(
                "{} ({}): {warning}",
                ctx.name,
                harness.display_name()
            ));
        }
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
                ctx.sealed,
                ctx.item_path,
                ctx.manifest,
                ItemKind::Agent,
                ctx.name,
                harness,
            )?,
            upstream_skills: Some(skills.upstream_now.clone()),
            artifact: Artifact::File {
                path: file,
                bytes: rendered.text.into_bytes(),
            },
        });
    }
    Ok(())
}
