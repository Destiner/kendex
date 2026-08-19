//! Declaring what a scope wants: items by name, whole sets by the name their
//! catalog offers them under, and the optional extras taken along the way.
//! Every check runs before the first declaration is written, so a request
//! that cannot be satisfied leaves the manifest exactly as it was.

use std::collections::BTreeSet;

use super::{ensure_manifest_persisted, manifest_for_mutation};
use crate::engine::{EngineReport, PlanOptions, plan_scope};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::lock::{Lock, lock_path};
use crate::manifest::{ItemDecl, Manifest, Method};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::{self, find_item, list_items, source_config};

#[derive(Debug, Default)]
pub struct AddRequest {
    /// v1 positional source: `owner/repo`, a path, or a declared source
    /// name. `None` means the default source.
    pub source: Option<String>,
    pub agents: Vec<String>,
    pub skills: Vec<String>,
    pub all: bool,
    pub harnesses: Option<Vec<HarnessId>>,
    pub copy: bool,
    pub no_auto_skills: bool,
    /// Optional dependencies to take, by name. The choice is recorded under
    /// every item from this source that offers one by that name.
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
    let mut manifest = manifest_for_mutation(env, scope)?;
    let source_name = ensure_source(&mut manifest, request.source.as_deref())?;
    let ready = source::require_ready(env, scope, &source_name, &manifest)?;
    let hold_at = hold_commit(request, &source_name, &ready)?;
    let sealed = crate::source_read::SealedSource::open(&ready.root)?;
    let config = source_config(&sealed, crate::source::repo_leaf(&ready.provenance))?;
    let lock = crate::lock::load(&lock_path(env, scope))?;

    let mut agents = request.agents.clone();
    let mut skills = request.skills.clone();
    if request.all {
        agents = list_items(&sealed, &config, ItemKind::Agent);
        skills = list_items(&sealed, &config, ItemKind::Skill);
    }
    for (kind, names) in [(ItemKind::Agent, &agents), (ItemKind::Skill, &skills)] {
        for name in names {
            if find_item(&sealed, &config, kind, name).is_none() {
                return Err(CoreError::ItemNotInSource {
                    name: name.clone(),
                    source_name: source_name.clone(),
                });
            }
        }
    }

    if !request.no_auto_skills {
        let available = list_items(&sealed, &config, ItemKind::Skill);
        let mut wanted: BTreeSet<String> = skills.iter().cloned().collect();
        for agent in &agents {
            let path = find_item(&sealed, &config, ItemKind::Agent, agent).ok_or_else(|| {
                CoreError::ItemNotInSource {
                    name: agent.clone(),
                    source_name: source_name.clone(),
                }
            })?;
            let text = sealed.read_if_exists(&path)?.unwrap_or_default();
            if let Ok(parsed) = crate::render::agent::parse_source_agent(&text) {
                for skill in
                    crate::mapping::upstream_skills(agent, parsed.role, &config, &available)
                {
                    wanted.insert(skill);
                }
            }
        }
        skills = wanted.into_iter().collect();
    }

    // Every check runs before the first declaration is written (invariant
    // 11): a choice naming an optional dependency nothing offers is an error
    // that leaves the manifest exactly as it was.
    let chosen = optional_choices(&sealed, &config, &manifest, &skills, &source_name, request)?;
    let mut sets = Vec::new();
    for name in &request.bundles {
        match crate::source::bundles::find(&sealed, &config, name)? {
            Some(bundle) => sets.push(bundle),
            None => {
                return Err(CoreError::NoSuchBundle {
                    name: name.clone(),
                    source_name: source_name.clone(),
                });
            }
        }
    }

    for bundle in sets {
        declare_bundle(
            &mut manifest,
            &bundle,
            &source_name,
            request,
            hold_at.as_deref(),
        );
    }

    for (kind, names) in [(ItemKind::Agent, agents), (ItemKind::Skill, skills)] {
        for name in names {
            declare(
                env,
                scope,
                &mut manifest,
                &lock,
                kind,
                &name,
                &source_name,
                request,
                hold_at.as_deref(),
            )?;
        }
    }
    for (parent, name) in chosen {
        let taken = manifest.optional_dependencies.entry(parent).or_default();
        if !taken.contains(&name) {
            taken.push(name);
            taken.sort();
        }
    }

    let mut report = plan_scope(env, scope, &manifest, &lock, &PlanOptions::default())?;
    ensure_manifest_persisted(env, scope, &manifest, &mut report)?;
    Ok(report)
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
) {
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
    for member in &bundle.members {
        if let Some(held) = manifest.suppressed.get_mut(&member.kind) {
            held.retain(|suppressed| suppressed != &member.name);
        }
    }
    manifest.suppressed.retain(|_, held| !held.is_empty());
}

/// The commit a `--hold` request freezes its declarations at. Only a
/// remote resolves to one; a hold on anything else is refused before the
/// first declaration is written (invariant 11).
fn hold_commit(
    request: &AddRequest,
    source_name: &str,
    ready: &crate::source::ResolvedSource,
) -> Result<Option<String>> {
    match (request.hold, &ready.commit) {
        (false, _) => Ok(None),
        (true, Some(commit)) => Ok(Some(commit.clone())),
        (true, None) => Err(CoreError::ItemRevUnsupported {
            source_name: source_name.to_owned(),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn declare(
    env: &Env,
    scope: &Scope,
    manifest: &mut Manifest,
    lock: &Lock,
    kind: ItemKind,
    name: &str,
    source_name: &str,
    request: &AddRequest,
    hold_at: Option<&str>,
) -> Result<()> {
    // Invariant 4: same-source redeclare is a no-op; a name already
    // installed from elsewhere is a hard error naming the original.
    for entry in lock.entries.values() {
        if entry.kind == kind && entry.name == name && entry.source != source_name {
            let requested = match source::resolve(env, scope, source_name, manifest)? {
                source::SourceState::Ready(ready) => ready.provenance,
                _ => source_name.to_owned(),
            };
            return Err(CoreError::SourceCollision {
                name: name.to_owned(),
                existing: entry.source_repo.clone(),
                requested,
            });
        }
    }
    let decl = manifest
        .declared_mut(kind)
        .entry(name.to_owned())
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
    // Asking for something back is the plainest possible statement that it
    // is wanted, so it outranks a removal recorded earlier.
    if let Some(held) = manifest.suppressed.get_mut(&kind) {
        held.retain(|suppressed| suppressed != name);
    }
    manifest.suppressed.retain(|_, held| !held.is_empty());
    Ok(())
}

/// Which item each chosen optional dependency belongs to. Choices are
/// recorded against the item that offers them, so a refresh knows what was
/// taken without having to guess from what is installed. A name nothing
/// offers is an error, not a silently ignored flag.
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
        let mut offered_by = Vec::new();
        for parent in &offers {
            let Some(dir) = find_item(sealed, config, ItemKind::Skill, parent) else {
                continue;
            };
            if crate::engine::deps::declared_dependencies(sealed, &dir)?
                .optional
                .contains(wanted)
            {
                offered_by.push(parent.clone());
            }
        }
        if offered_by.is_empty() {
            return Err(CoreError::NoSuchOptional {
                name: wanted.clone(),
                source_name: source_name.to_owned(),
            });
        }
        chosen.extend(
            offered_by
                .into_iter()
                .map(|parent| (parent, wanted.clone())),
        );
    }
    Ok(chosen)
}

/// Map a CLI source argument to a declared source name, declaring it when
/// new. References parse through [`crate::source_ref::parse_typed`], and a
/// repository already subscribed under any spelling reuses that
/// subscription.
mod pick;
use pick::ensure_source;
