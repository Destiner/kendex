use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::{LOCAL_SOURCE_NAME, Manifest};
use crate::model::{ItemKind, Scope};
use crate::source_read::SealedSource;

/// A source the engine can read right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    pub name: String,
    pub root: PathBuf,
    /// Durable provenance: `owner/repo`, a canonical path, or `local`.
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceState {
    Ready(ResolvedSource),
    /// Declared remote whose cache does not exist yet — not an error until
    /// something needs its content (remote resolution arrives in Phase 5).
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
            })),
            _ => Ok(SourceState::Missing {
                name: name.to_owned(),
                path: joined,
            }),
        };
    }
    if let Some(repo) = &decl.repo {
        let cache = env.source_cache_dir().join(repo.replace('/', "_"));
        if cache.is_dir() {
            return Ok(SourceState::Ready(ResolvedSource {
                name: name.to_owned(),
                root: cache,
                provenance: repo.clone(),
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
}

pub fn source_config(sealed: &SealedSource) -> Result<SourceConfig> {
    let mut config = SourceConfig {
        agent_dirs: vec!["agents".to_owned()],
        skill_dirs: vec!["skills".to_owned()],
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
