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

fn detected_harnesses(env: &Env) -> Vec<HarnessId> {
    crate::harness::all_adapters()
        .iter()
        .filter_map(|a| {
            a.detect(env, &a.default_global_root(env))
                .map(|found| found.harness)
        })
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
        expand_dependencies(&sealed, &config, &mut wanted);
        skills = wanted.into_iter().collect();
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
            )?;
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
    Ok(())
}

/// Transitive closure over `dependencies.required` in skill frontmatter.
fn expand_dependencies(
    sealed: &crate::source_read::SealedSource,
    config: &crate::source::SourceConfig,
    wanted: &mut BTreeSet<String>,
) {
    let mut queue: Vec<String> = wanted.iter().cloned().collect();
    while let Some(name) = queue.pop() {
        let Some(dir) = find_item(sealed, config, ItemKind::Skill, &name) else {
            continue;
        };
        let Ok(Some(text)) = sealed.read_if_exists(&dir.join("SKILL.md")) else {
            continue;
        };
        for dep in required_dependencies(&text) {
            if wanted.insert(dep.clone()) {
                queue.push(dep);
            }
        }
    }
}

/// `dependencies: {required: [a, b]}` — flat and nested YAML list forms.
fn required_dependencies(skill_md: &str) -> Vec<String> {
    let Some(front) = skill_md
        .strip_prefix("---")
        .and_then(|rest| rest.find("\n---").map(|end| &rest[..end]))
    else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    let mut in_dependencies = false;
    let mut in_required = false;
    for line in front.lines() {
        let trimmed = line.trim();
        if !line.starts_with(' ') {
            in_dependencies = trimmed.starts_with("dependencies:");
            in_required = false;
            continue;
        }
        if !in_dependencies {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("required:") {
            let rest = rest.trim();
            if let Some(list) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                deps.extend(
                    list.split(',')
                        .map(|s| s.trim().trim_matches('"').to_owned())
                        .filter(|s| !s.is_empty()),
                );
                in_required = false;
            } else {
                in_required = true;
            }
            continue;
        }
        if trimmed.starts_with("optional:") {
            in_required = false;
            continue;
        }
        if in_required && let Some(item) = trimmed.strip_prefix("- ") {
            deps.push(item.trim().trim_matches('"').to_owned());
        }
    }
    deps
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
            enabled: true,
        }
    } else {
        SourceDecl {
            repo: None,
            path: Some(requested.to_owned()),
            enabled: true,
        }
    };
    manifest.sources.insert(name.clone(), decl);
    Ok(name)
}

/// Drop declarations and plan the removal of exactly those items.
pub fn remove(env: &Env, scope: &Scope, names: &[String]) -> Result<EngineReport> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let lock = crate::lock::load(&lock_path(env, scope))?;
    for name in names {
        for kind in DECLARED_KINDS {
            manifest.declared_mut(kind).remove(name);
        }
        manifest.plugins.remove(name);
        manifest.agent_skills.remove(name);
        manifest.skill_instructions.remove(name);
    }
    let options = PlanOptions {
        remove_orphans: true,
        removal_filter: Some(names.to_vec()),
    };
    let mut report = plan_scope(env, scope, &manifest, &lock, &options)?;
    ensure_manifest_persisted(env, scope, &manifest, &mut report)?;
    Ok(report)
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

/// The plan must persist the mutated manifest exactly once; plan_scope adds
/// its own write only when upstream skill merges changed it further.
fn ensure_manifest_persisted(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    report: &mut EngineReport,
) -> Result<()> {
    let already = report
        .plan
        .ops
        .iter()
        .any(|op| matches!(op.op, crate::apply::Op::WriteManifest { .. }));
    if already {
        return Ok(());
    }
    let path = manifest::manifest_path(env, scope);
    report.plan.ops.insert(
        0,
        crate::apply::PlannedOp {
            description: "Save vstack.toml".into(),
            op: crate::apply::Op::WriteManifest {
                pre: crate::apply::Pre::observed(&path)?,
                path,
                manifest: Box::new(manifest.clone()),
            },
        },
    );
    Ok(())
}
