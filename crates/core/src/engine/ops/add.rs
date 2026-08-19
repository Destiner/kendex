//! Declaring what a scope wants: items by name, whole sets by the name their
//! catalog offers them under, and the optional extras taken along the way.
//! Every check runs before anything is persisted, so a request that cannot
//! be satisfied leaves the manifest exactly as it was.

use std::collections::BTreeSet;

use super::{ensure_manifest_persisted, manifest_for_mutation};
use crate::engine::{EngineReport, PlanOptions, plan_scope};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::lock::{Lock, lock_path};
use crate::manifest::{ItemDecl, Manifest, Method};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::{self, find_item, list_items, source_config};

#[derive(Debug, Default, Clone)]
pub struct AddRequest {
    /// v1 positional source: `owner/repo`, a path, or a declared source
    /// name. `None` sends bare names through the cross-subscription
    /// search; a `marketplace::name` spelling names its subscription
    /// itself.
    pub source: Option<String>,
    pub agents: Vec<String>,
    pub skills: Vec<String>,
    pub hooks: Vec<String>,
    pub commands: Vec<String>,
    pub mcp_servers: Vec<String>,
    /// Always refused: a Pi extension installs with the bundle that
    /// carries it, never on its own. The field exists so every shell gets
    /// the same refusal from the engine.
    pub pi_extensions: Vec<String>,
    pub all: bool,
    pub harnesses: Option<Vec<HarnessId>>,
    pub copy: bool,
    pub no_auto_skills: bool,
    /// Optional dependencies to take, by name. The choice is recorded under
    /// every item this request touches that offers one by that name.
    pub optional: Vec<String>,
    /// Curated sets to install whole, by the name the catalog offers them
    /// under. What each holds derives at plan time; the manifest records only
    /// that the set is installed.
    pub bundles: Vec<String>,
    /// Hold every declaration this request writes at the commit the source
    /// resolves to right now — "manual updates" from the first moment. A
    /// hold on a source without revisions (a path, local) is refused before
    /// anything is written.
    pub hold: bool,
}

/// Declare items (and their auto-expanded skills), then plan the scope.
/// The returned report's plan includes persisting the updated manifest.
pub fn add(env: &Env, scope: &Scope, request: &AddRequest) -> Result<EngineReport> {
    add_seeded(env, scope, request, None)
}

/// `add`, optionally declaring a subscription into the scope first. Installing
/// into a project from a personal subscription seeds that subscription here so
/// the single plan writes the subscription and the packages together: if the
/// add is refused, nothing is persisted, and the project is never left carrying
/// a subscription it never installed anything from.
pub fn add_seeded(
    env: &Env,
    scope: &Scope,
    request: &AddRequest,
    seed: Option<(String, crate::manifest::SourceDecl)>,
) -> Result<EngineReport> {
    if let Some(name) = request.pi_extensions.first() {
        return Err(CoreError::PiExtensionDirect { name: name.clone() });
    }
    let mut manifest = manifest_for_mutation(env, scope)?;
    if let Some((name, decl)) = seed {
        manifest.sources.insert(name, decl);
    }
    let lock = crate::lock::load(&lock_path(env, scope))?;
    let (mut groups, context) = place::place(env, scope, &mut manifest, request)?;
    let all_source = match (request.all, &context) {
        (false, _) => None,
        (true, Some(ctx)) => Some(ctx.clone()),
        (true, None) => Some(pick::default_source(&manifest)?),
    };
    if let Some(source_name) = &all_source {
        groups.entry(source_name.clone()).or_default();
    }

    let mut notes = Vec::new();
    let mut optional_offers: Vec<(String, String)> = Vec::new();
    for (source_name, wanted) in &groups {
        add_from(
            env,
            scope,
            &mut manifest,
            &lock,
            request,
            source_name,
            wanted,
            all_source.as_deref() == Some(source_name),
            &mut notes,
            &mut optional_offers,
        )?;
    }
    // A choice naming an optional dependency nothing offers is an error
    // that leaves the manifest exactly as it was — never a silently
    // ignored flag.
    for wanted in &request.optional {
        if !optional_offers.iter().any(|(_, name)| name == wanted) {
            return Err(CoreError::NoSuchOptional {
                name: wanted.clone(),
                source_name: groups.keys().cloned().collect::<Vec<_>>().join(", "),
            });
        }
    }
    for (parent, name) in optional_offers {
        let taken = manifest.optional_dependencies.entry(parent).or_default();
        if !taken.contains(&name) {
            taken.push(name);
            taken.sort();
        }
    }

    let mut report = plan_scope(env, scope, &manifest, &lock, &PlanOptions::default())?;
    report.notes.extend(notes);
    ensure_manifest_persisted(env, scope, &manifest, &mut report)?;
    Ok(report)
}

/// Everything this request takes from one subscription: existence checks,
/// agent-to-skill expansion, item declarations, then bundles — bundles
/// last, so installing a whole set can subsume the members it now
/// accounts for.
#[allow(clippy::too_many_arguments)]
fn add_from(
    env: &Env,
    scope: &Scope,
    manifest: &mut Manifest,
    lock: &Lock,
    request: &AddRequest,
    source_name: &str,
    wanted: &place::Wanted,
    take_all: bool,
    notes: &mut Vec<String>,
    optional_offers: &mut Vec<(String, String)>,
) -> Result<()> {
    let ready = source::require_ready(env, scope, source_name, manifest)?;
    let hold_at = hold_commit(request, source_name, &ready)?;
    let sealed = crate::source_read::SealedSource::open(&ready.root)?;
    let config = source_config(&sealed, crate::source::repo_leaf(&ready.provenance))?;

    let mut agents = wanted.agents.clone();
    let mut skills = wanted.skills.clone();
    let mut hooks = wanted.hooks.clone();
    let mut commands = wanted.commands.clone();
    let mut mcp_servers = wanted.mcp_servers.clone();
    if take_all {
        agents = list_items(&sealed, &config, ItemKind::Agent);
        skills = list_items(&sealed, &config, ItemKind::Skill);
        hooks = list_items(&sealed, &config, ItemKind::Hook);
        commands = list_items(&sealed, &config, ItemKind::Command);
        mcp_servers = list_items(&sealed, &config, ItemKind::McpServer);
    }
    for (kind, names) in [
        (ItemKind::Agent, &agents),
        (ItemKind::Skill, &skills),
        (ItemKind::Hook, &hooks),
        (ItemKind::Command, &commands),
        (ItemKind::McpServer, &mcp_servers),
    ] {
        for name in names {
            if find_item(&sealed, &config, kind, name).is_none() {
                return Err(CoreError::ItemNotInSource {
                    name: name.clone(),
                    source_name: source_name.to_owned(),
                });
            }
        }
    }

    if !request.no_auto_skills {
        let available = list_items(&sealed, &config, ItemKind::Skill);
        let mut expanded: BTreeSet<String> = skills.iter().cloned().collect();
        for agent in &agents {
            let path = find_item(&sealed, &config, ItemKind::Agent, agent).ok_or_else(|| {
                CoreError::ItemNotInSource {
                    name: agent.clone(),
                    source_name: source_name.to_owned(),
                }
            })?;
            let text = sealed.read_if_exists(&path)?.unwrap_or_default();
            if let Ok(parsed) = crate::render::agent::parse_source_agent(&text) {
                for skill in
                    crate::mapping::upstream_skills(agent, parsed.role, &config, &available)
                {
                    expanded.insert(skill);
                }
            }
        }
        skills = expanded.into_iter().collect();
    }

    optional_offers.extend(optional_choices(
        &sealed,
        &config,
        manifest,
        &skills,
        source_name,
        request,
    )?);
    let mut sets = Vec::new();
    for name in &wanted.bundles {
        match crate::source::bundles::find(&sealed, &config, name)? {
            Some(bundle) => sets.push(bundle),
            None => {
                return Err(CoreError::NoSuchBundle {
                    name: name.clone(),
                    source_name: source_name.to_owned(),
                });
            }
        }
    }

    // Bundles first: declaring a set folds in the equal-option members
    // declared earlier, while an item this same request asks for by name
    // is declared after — asking for both is asking for both.
    for bundle in sets {
        subsume::require_free(manifest, &bundle.name, source_name)?;
        let decl = declare_bundle(manifest, &bundle, source_name, request, hold_at.as_deref());
        subsume::subsume(manifest, &bundle, &decl, notes);
    }
    for (kind, names) in [
        (ItemKind::Agent, agents),
        (ItemKind::Skill, skills),
        (ItemKind::Hook, hooks),
        (ItemKind::Command, commands),
        (ItemKind::McpServer, mcp_servers),
    ] {
        for name in names {
            declare(
                env,
                scope,
                manifest,
                lock,
                kind,
                &name,
                source_name,
                request,
                hold_at.as_deref(),
            )?;
        }
    }
    Ok(())
}

/// Declare one curated set, carried the way the request asked. Asking for
/// the set is asking for all of it: a member held back by an earlier
/// removal comes with it, the same way asking for an item again outranks
/// the removal that took it away.
fn declare_bundle(
    manifest: &mut Manifest,
    bundle: &crate::source::CatalogBundle,
    source_name: &str,
    request: &AddRequest,
    hold_at: Option<&str>,
) -> ItemDecl {
    let decl = manifest
        .bundles
        .entry(bundle.name.clone())
        .or_insert_with(|| ItemDecl::from_source(source_name));
    decl.source = source_name.to_owned();
    if let Some(harnesses) = &request.harnesses {
        decl.harnesses = Some(harnesses.clone());
    }
    if request.copy {
        decl.method = Some(Method::Copy);
    }
    if let Some(commit) = hold_at {
        decl.rev = Some(commit.to_owned());
    }
    let declared = decl.clone();
    for member in &bundle.members {
        if let Some(held) = manifest.suppressed.get_mut(&member.kind) {
            held.retain(|suppressed| suppressed != &member.name);
        }
    }
    manifest.suppressed.retain(|_, held| !held.is_empty());
    declared
}

/// Which item each chosen optional dependency belongs to. Choices are
/// recorded against the item that offers them, so a refresh knows what was
/// taken without having to guess from what is installed. A name no
/// subscription this request touches offers is an error — raised by the
/// caller once every subscription has answered.
fn optional_choices(
    sealed: &crate::source_read::SealedSource,
    config: &crate::source::SourceConfig,
    manifest: &Manifest,
    adding: &[String],
    source_name: &str,
    request: &AddRequest,
) -> Result<Vec<(String, String)>> {
    if request.optional.is_empty() {
        return Ok(Vec::new());
    }
    let mut offers: BTreeSet<String> = adding.iter().cloned().collect();
    offers.extend(
        manifest
            .skills
            .iter()
            .filter(|(_, decl)| decl.source == source_name)
            .map(|(name, _)| name.clone()),
    );
    let mut chosen = Vec::new();
    for wanted in &request.optional {
        for parent in &offers {
            let Some(dir) = find_item(sealed, config, ItemKind::Skill, parent) else {
                continue;
            };
            if crate::engine::deps::declared_dependencies(sealed, &dir)?
                .optional
                .contains(wanted)
            {
                chosen.push((parent.clone(), wanted.clone()));
            }
        }
    }
    Ok(chosen)
}

/// Map a CLI source argument to a declared source name, declaring it when
/// new. References parse through [`crate::source_ref::parse_typed`], and a
/// repository already subscribed under any spelling reuses that
/// subscription.
mod declare;
mod pick;
mod place;
mod subsume;

use declare::{declare, hold_commit};
