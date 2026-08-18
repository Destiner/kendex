use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::{LOCAL_SOURCE_NAME, Manifest, SourceDecl};
use crate::model::{ItemKind, Scope};
use crate::source_read::SealedSource;

pub mod bundles;
mod catalog;
mod plugin_registry;

pub use bundles::CatalogBundle;
pub use catalog::{CatalogGroup, CatalogItem, CatalogMetadata, metadata as catalog_metadata};
pub use plugin_registry::{CatalogFinding, PluginEntry, Registry};

/// A source the engine can read right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    pub name: String,
    pub root: PathBuf,
    /// Durable provenance: `owner/repo`, a canonical path, or `local`.
    pub provenance: String,
    /// Remotes only: the commit this root holds. The root is that commit's
    /// own directory, so it cannot change while it is being read.
    pub commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceState {
    Ready(ResolvedSource),
    /// Declared remote the cache cannot serve yet — not an error until
    /// something needs its content. A refresh fetches it.
    Pending {
        name: String,
        repo: String,
    },
    Disabled {
        name: String,
    },
    Missing {
        name: String,
        path: PathBuf,
    },
}

/// Where adopted content lives for a scope — always catalog-shaped.
pub fn local_source_root(env: &Env, scope: &Scope) -> PathBuf {
    match scope {
        Scope::Global => env.global_local_source_dir(),
        Scope::Project { root } => root.join(".vstack-local"),
    }
}

pub fn resolve(env: &Env, scope: &Scope, name: &str, manifest: &Manifest) -> Result<SourceState> {
    if name == LOCAL_SOURCE_NAME {
        // Adopt creates this root; until then the reserved source has no
        // content and reads as missing, never as an open-able Ready root.
        let root = local_source_root(env, scope);
        if !root.is_dir() {
            return Ok(SourceState::Missing {
                name: name.to_owned(),
                path: root,
            });
        }
        return Ok(SourceState::Ready(ResolvedSource {
            name: name.to_owned(),
            root,
            provenance: LOCAL_SOURCE_NAME.to_owned(),
            commit: None,
        }));
    }
    let Some(decl) = manifest.sources.get(name) else {
        return Err(CoreError::UnknownSource {
            name: name.to_owned(),
        });
    };
    if !decl.enabled {
        return Ok(SourceState::Disabled {
            name: name.to_owned(),
        });
    }
    if let Some(path) = &decl.path {
        let base = match scope {
            Scope::Global => env.home.clone(),
            Scope::Project { root } => root.clone(),
        };
        let joined = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            base.join(path)
        };
        return match joined.canonicalize() {
            Ok(root) if root.is_dir() => Ok(SourceState::Ready(ResolvedSource {
                name: name.to_owned(),
                provenance: root.display().to_string(),
                root,
                commit: None,
            })),
            _ => Ok(SourceState::Missing {
                name: name.to_owned(),
                path: joined,
            }),
        };
    }
    if let Some(repo) = &decl.repo {
        if let Some(resolution) = crate::remote::cached(env, repo, decl.rev.as_deref())? {
            return Ok(SourceState::Ready(ResolvedSource {
                name: name.to_owned(),
                root: resolution.root,
                provenance: repo.clone(),
                commit: Some(resolution.commit),
            }));
        }
        // Last resort: the commit this scope last resolved to. A tag that
        // has since been deleted upstream, or a mirror that was cleaned
        // away, still leaves the installed commit readable here — and the
        // record knows which commit that is, so the answer carries it
        // rather than letting a later lock write erase an honest one.
        if let Some((root, commit)) = last_resolved(env, scope, name, repo, decl) {
            return Ok(SourceState::Ready(ResolvedSource {
                name: name.to_owned(),
                root,
                provenance: repo.clone(),
                commit: Some(commit),
            }));
        }
        return Ok(SourceState::Pending {
            name: name.to_owned(),
            repo: repo.clone(),
        });
    }
    Err(CoreError::UnknownSource {
        name: name.to_owned(),
    })
}

/// The checkout for the commit this scope's lock recorded, if the cache
/// still holds it unmodified. Only for the declaration that produced it: a
/// manifest that now names another repository or another revision must not
/// be served the previous one under the new one's name.
fn last_resolved(
    env: &Env,
    scope: &Scope,
    name: &str,
    repo: &str,
    decl: &SourceDecl,
) -> Option<(PathBuf, String)> {
    let lock = crate::lock::load(&crate::lock::lock_path(env, scope)).ok()?;
    let recorded = lock.sources.get(name)?;
    if recorded.repo != repo || recorded.rev != decl.rev {
        return None;
    }
    let key = crate::remote::store::repo_key(&crate::remote::clone_url(env, &recorded.repo));
    let root = crate::remote::store::published(env, &key, &recorded.commit)?;
    Some((root, recorded.commit.clone()))
}

/// Like [`resolve`], but honoring an item-level revision override: the
/// item's `rev` outranks the source's. Only a repo source has revisions —
/// a rev naming a path or local source is refused with the fix in hand.
/// The lock's last-resolved fallback is deliberately skipped: it records
/// what the *source declaration* produced, which says nothing about an
/// item pinned somewhere else in history.
pub fn resolve_at(
    env: &Env,
    scope: &Scope,
    name: &str,
    manifest: &Manifest,
    rev: Option<&str>,
) -> Result<SourceState> {
    let Some(rev) = rev else {
        return resolve(env, scope, name, manifest);
    };
    if name == LOCAL_SOURCE_NAME {
        return Err(CoreError::ItemRevUnsupported {
            source_name: name.to_owned(),
        });
    }
    let Some(decl) = manifest.sources.get(name) else {
        return Err(CoreError::UnknownSource {
            name: name.to_owned(),
        });
    };
    if !decl.enabled {
        return Ok(SourceState::Disabled {
            name: name.to_owned(),
        });
    }
    let Some(repo) = &decl.repo else {
        return Err(CoreError::ItemRevUnsupported {
            source_name: name.to_owned(),
        });
    };
    match crate::remote::cached(env, repo, Some(rev))? {
        Some(resolution) => Ok(SourceState::Ready(ResolvedSource {
            name: name.to_owned(),
            root: resolution.root,
            provenance: repo.clone(),
            commit: Some(resolution.commit),
        })),
        None => Ok(SourceState::Pending {
            name: name.to_owned(),
            repo: repo.clone(),
        }),
    }
}

/// A source's ready root, or the error that explains why content is
/// unreachable — for operations that need bytes now.
pub fn require_ready(
    env: &Env,
    scope: &Scope,
    name: &str,
    manifest: &Manifest,
) -> Result<ResolvedSource> {
    match resolve(env, scope, name, manifest)? {
        SourceState::Ready(source) => Ok(source),
        SourceState::Pending { name, .. } => Err(CoreError::SourcePending { name }),
        SourceState::Disabled { name } => Err(CoreError::SourceDisabled { name }),
        SourceState::Missing { name, path } => Err(CoreError::SourceMissing { name, path }),
    }
}

/// Source-side layout + mapping tables, read leniently — source catalogs are
/// v1-format repos (no schema key) and stay valid forever.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceConfig {
    pub agent_dirs: Vec<String>,
    pub skill_dirs: Vec<String>,
    pub agent_skills: BTreeMap<String, Vec<String>>,
    pub role_skills: BTreeMap<String, Vec<String>>,
    pub frontmatter: BTreeMap<String, BTreeMap<String, crate::manifest::FrontmatterOverrides>>,
    /// The curated sets this catalog offers by name. Empty for a
    /// plugin-registry-shaped catalog, whose plugins are its sets.
    pub bundles: BTreeMap<String, CatalogBundle>,
    /// Set when the source carries a plugin registry: its items live
    /// one plugin deep and are named `<plugin>/<item>`. The kind directories
    /// at the root are not read in that case — the registry says what the
    /// catalog offers, and reading both would offer the same file twice.
    pub plugin_registry: Option<Registry>,
}

impl SourceConfig {
    /// Everything wrong with the catalog's own registry, if it has one.
    pub fn findings(&self) -> &[CatalogFinding] {
        match &self.plugin_registry {
            Some(registry) => &registry.findings,
            None => &[],
        }
    }
}

pub fn source_config(sealed: &SealedSource) -> Result<SourceConfig> {
    let mut config = SourceConfig {
        agent_dirs: vec!["agents".to_owned()],
        skill_dirs: vec!["skills".to_owned()],
        plugin_registry: plugin_registry::read(sealed)?,
        ..SourceConfig::default()
    };
    let Some(text) = sealed.read_if_exists(&sealed.root().join("vstack.toml"))? else {
        return Ok(config);
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return Ok(config);
    };
    if let Some(catalog) = table.get("catalog").and_then(|c| c.as_table()) {
        if let Some(dirs) = string_list(catalog.get("agents")) {
            config.agent_dirs = dirs;
        }
        if let Some(dirs) = string_list(catalog.get("skills")) {
            config.skill_dirs = dirs;
        }
    }
    config.bundles = bundles::declared(&table);
    if let Some(mapping) = table.get("agent-skills").and_then(|t| t.as_table()) {
        for (agent, skills) in mapping {
            if let Some(list) = string_list(Some(skills)) {
                config.agent_skills.insert(agent.clone(), list);
            }
        }
    }
    if let Some(mapping) = table.get("role-skills").and_then(|t| t.as_table()) {
        for (role, skills) in mapping {
            if let Some(list) = string_list(Some(skills)) {
                config.role_skills.insert(role.clone(), list);
            }
        }
    }
    if let Some(frontmatter) = table.get("agent-frontmatter").and_then(|t| t.as_table()) {
        for (harness, agents) in frontmatter {
            let Some(agents) = agents.as_table() else {
                continue;
            };
            let mut per_agent = BTreeMap::new();
            for (agent, overrides) in agents {
                if let Ok(parsed) = overrides.clone().try_into() {
                    per_agent.insert(agent.clone(), parsed);
                }
            }
            config.frontmatter.insert(harness.clone(), per_agent);
        }
    }
    Ok(config)
}

fn string_list(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value?.as_array().map(|list| {
        list.iter()
            .filter_map(|v| v.as_str())
            .map(str::to_owned)
            .collect()
    })
}

pub fn find_item(
    sealed: &SealedSource,
    config: &SourceConfig,
    kind: ItemKind,
    name: &str,
) -> Option<PathBuf> {
    // A name that cannot be a file is not on offer, whoever asks — a bundle
    // member or a dependency is not checked anywhere else.
    if crate::names::item_problem(name).is_some() {
        return None;
    }
    if let Some(registry) = &config.plugin_registry {
        return catalog::find(sealed, registry, kind, name);
    }
    let root = sealed.root();
    match kind {
        ItemKind::Skill => config
            .skill_dirs
            .iter()
            .map(|d| root.join(d).join(name))
            .find(|p| sealed.is_file(&p.join("SKILL.md"))),
        ItemKind::Agent => config
            .agent_dirs
            .iter()
            .map(|d| root.join(d).join(format!("{name}.md")))
            .find(|p| sealed.is_file(p)),
        ItemKind::Hook => catalog_file(sealed, "hooks", &format!("{name}.sh")),
        ItemKind::Command => catalog_file(sealed, "commands", &format!("{name}.md")),
        ItemKind::McpServer => catalog_file(sealed, "mcp", &format!("{name}.toml")),
        ItemKind::Plugin | ItemKind::PiExtension => None,
    }
}

fn catalog_file(sealed: &SealedSource, dir: &str, file: &str) -> Option<PathBuf> {
    let path = sealed.root().join(dir).join(file);
    sealed.is_file(&path).then_some(path)
}

pub fn list_items(sealed: &SealedSource, config: &SourceConfig, kind: ItemKind) -> Vec<String> {
    if let Some(registry) = &config.plugin_registry {
        return catalog::items(sealed, registry, kind);
    }
    let mut names = Vec::new();
    match kind {
        ItemKind::Skill => {
            for dir in &config.skill_dirs {
                let Ok(entries) = sealed.list_dir(&sealed.root().join(dir)) else {
                    continue;
                };
                for path in entries {
                    if sealed.is_file(&path.join("SKILL.md"))
                        && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    {
                        names.push(name.to_owned());
                    }
                }
            }
        }
        ItemKind::Agent => {
            for dir in &config.agent_dirs {
                let Ok(entries) = sealed.list_dir(&sealed.root().join(dir)) else {
                    continue;
                };
                for path in entries {
                    if path.extension().is_some_and(|e| e == "md")
                        && sealed.is_file(&path)
                        && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    {
                        names.push(stem.to_owned());
                    }
                }
            }
        }
        _ => {}
    }
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests;
