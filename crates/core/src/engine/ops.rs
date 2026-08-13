use std::collections::BTreeSet;

use super::{EngineReport, PlanOptions, plan_scope};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::lock::{Lock, lock_path};
use crate::manifest::{
    self, DEFAULT_SOURCE_NAME, ItemDecl, LOCAL_SOURCE_NAME, Manifest, Method, SourceDecl,
};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::{self, find_item, list_items, source_config};

mod persist;
use persist::ensure_manifest_persisted;

/// Every kind a manifest declares by name. Plugins are excluded: they carry
/// only an enabled flag, in their own table.
const DECLARED_KINDS: [ItemKind; 6] = [
    ItemKind::Agent,
    ItemKind::Skill,
    ItemKind::Hook,
    ItemKind::Command,
    ItemKind::McpServer,
    ItemKind::PiExtension,
];

/// The tools on this machine a fresh manifest should install to — a tool
/// vstack can only read is detected and listed, never seeded as a target
/// whose every install would silently do nothing.
fn detected_harnesses(env: &Env) -> Vec<HarnessId> {
    crate::harness::all_adapters()
        .iter()
        .filter_map(|a| {
            a.detect(env, &a.default_global_root(env))
                .map(|found| found.harness)
        })
        .filter(|harness| crate::harness::installable(*harness))
        .collect()
}

/// Load the scope's manifest for mutation, seeding a fresh one (with the
/// default source) when none exists. Legacy files are a hard error.
pub fn manifest_for_mutation(env: &Env, scope: &Scope) -> Result<Manifest> {
    let path = manifest::manifest_path(env, scope);
    match manifest::load_for_mutation(&path)? {
        Some(manifest) => Ok(manifest),
        None => Ok(manifest::seed(&detected_harnesses(env))),
    }
}

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
}

/// Declare items (and their auto-expanded skills), then plan the scope.
/// The returned report's plan includes persisting the updated manifest.
pub fn add(env: &Env, scope: &Scope, request: &AddRequest) -> Result<EngineReport> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let source_name = ensure_source(&mut manifest, request.source.as_deref())?;
    let ready = source::require_ready(env, scope, &source_name, &manifest)?;
    let sealed = crate::source_read::SealedSource::open(&ready.root)?;
    let config = source_config(&sealed)?;
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
            if super::deps::declared_dependencies(sealed, &dir)?
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
/// new. `owner/repo` shapes become repo sources; anything else a path.
fn ensure_source(manifest: &mut Manifest, requested: Option<&str>) -> Result<String> {
    let Some(requested) = requested else {
        if manifest.sources.contains_key(DEFAULT_SOURCE_NAME) {
            return Ok(DEFAULT_SOURCE_NAME.to_owned());
        }
        if let Some(name) = manifest.sources.keys().next() {
            return Ok(name.clone());
        }
        return Err(CoreError::UnknownSource {
            name: DEFAULT_SOURCE_NAME.to_owned(),
        });
    };
    if requested == LOCAL_SOURCE_NAME || manifest.sources.contains_key(requested) {
        return Ok(requested.to_owned());
    }
    let is_repo = requested.contains('/')
        && !requested.starts_with('.')
        && !requested.starts_with('/')
        && !requested.starts_with('~')
        && requested.matches('/').count() == 1;
    for (name, decl) in &manifest.sources {
        let matches = if is_repo {
            decl.repo.as_deref() == Some(requested)
        } else {
            decl.path.as_deref() == Some(requested)
        };
        if matches {
            return Ok(name.clone());
        }
    }
    let base = requested
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(requested)
        .to_owned();
    let mut name = base.clone();
    let mut counter = 2;
    while manifest.sources.contains_key(&name) {
        name = format!("{base}-{counter}");
        counter += 1;
    }
    let decl = if is_repo {
        SourceDecl {
            repo: Some(requested.to_owned()),
            path: None,
            rev: None,
            enabled: true,
        }
    } else {
        SourceDecl {
            repo: None,
            path: Some(requested.to_owned()),
            rev: None,
            enabled: true,
        }
    };
    manifest.sources.insert(name.clone(), decl);
    Ok(name)
}

/// Drop declarations and plan the removal of exactly those items. A removal
/// is durable: an item something else still requires is written down as
/// suppressed rather than re-derived on the next plan, and every item that
/// requires it says so in the audit instead of quietly getting it back.
/// `sweep` also removes what nothing needs anymore — the dependencies whose
/// last dependent is going away.
pub fn remove(env: &Env, scope: &Scope, names: &[String], sweep: bool) -> Result<EngineReport> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let lock = crate::lock::load(&lock_path(env, scope))?;
    for name in names {
        for kind in DECLARED_KINDS {
            manifest.declared_mut(kind).remove(name);
        }
        manifest.plugins.remove(name);
        manifest.agent_skills.remove(name);
        manifest.skill_instructions.remove(name);
        manifest.optional_dependencies.remove(name);
        // Taking an item away also un-takes it wherever it was chosen as an
        // optional extra: that choice is the whole reason it would return.
        for taken in manifest.optional_dependencies.values_mut() {
            taken.retain(|chosen| chosen != name);
        }
    }
    manifest.optional_dependencies.retain(|_, t| !t.is_empty());
    for name in still_required(env, scope, &manifest, names) {
        manifest.suppress(ItemKind::Skill, &name);
    }
    let options = PlanOptions {
        remove_orphans: true,
        removal_filter: Some(names.to_vec()),
        sweep_unneeded: sweep,
    };
    let mut report = plan_scope(env, scope, &manifest, &lock, &options)?;
    ensure_manifest_persisted(env, scope, &manifest, &mut report)?;
    Ok(report)
}

/// Which of these names something that stays would pull straight back in.
fn still_required(env: &Env, scope: &Scope, manifest: &Manifest, names: &[String]) -> Vec<String> {
    let mut state = crate::engine::desired::DesiredState::default();
    let expansion = super::deps::expand(env, scope, manifest, &mut state);
    names
        .iter()
        .filter(|name| expansion.items.contains_key(*name))
        .cloned()
        .collect()
}

/// Flip declarations; disabling is non-destructive (invariant 5).
pub fn toggle(env: &Env, scope: &Scope, names: &[String], enabled: bool) -> Result<EngineReport> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let lock = crate::lock::load(&lock_path(env, scope))?;
    for name in names {
        for kind in DECLARED_KINDS {
            if let Some(decl) = manifest.declared_mut(kind).get_mut(name) {
                decl.enabled = enabled;
            }
        }
        if let Some(plugin) = manifest.plugins.get_mut(name) {
            plugin.enabled = enabled;
        }
    }
    let mut report = plan_scope(env, scope, &manifest, &lock, &PlanOptions::default())?;
    ensure_manifest_persisted(env, scope, &manifest, &mut report)?;
    Ok(report)
}
