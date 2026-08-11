use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::read_if_exists;
use crate::manifest::{LOCAL_SOURCE_NAME, Manifest};
use crate::model::{ItemKind, Scope};

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
        return Ok(SourceState::Ready(ResolvedSource {
            name: name.to_owned(),
            root: local_source_root(env, scope),
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

pub fn source_config(root: &Path) -> Result<SourceConfig> {
    let mut config = SourceConfig {
        agent_dirs: vec!["agents".to_owned()],
        skill_dirs: vec!["skills".to_owned()],
        ..SourceConfig::default()
    };
    let Some(text) = read_if_exists(&root.join("vstack.toml"))? else {
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
    root: &Path,
    config: &SourceConfig,
    kind: ItemKind,
    name: &str,
) -> Option<PathBuf> {
    match kind {
        ItemKind::Skill => config
            .skill_dirs
            .iter()
            .map(|d| root.join(d).join(name))
            .find(|p| p.join("SKILL.md").is_file()),
        ItemKind::Agent => config
            .agent_dirs
            .iter()
            .map(|d| root.join(d).join(format!("{name}.md")))
            .find(|p| p.is_file()),
        ItemKind::Hook => catalog_file(root, "hooks", &format!("{name}.sh")),
        ItemKind::Command => catalog_file(root, "commands", &format!("{name}.md")),
        ItemKind::McpServer => catalog_file(root, "mcp", &format!("{name}.toml")),
        ItemKind::Plugin | ItemKind::PiExtension => None,
    }
}

fn catalog_file(root: &Path, dir: &str, file: &str) -> Option<PathBuf> {
    let path = root.join(dir).join(file);
    path.is_file().then_some(path)
}

pub fn list_items(root: &Path, config: &SourceConfig, kind: ItemKind) -> Vec<String> {
    let mut names = Vec::new();
    match kind {
        ItemKind::Skill => {
            for dir in &config.skill_dirs {
                let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.join("SKILL.md").is_file()
                        && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    {
                        names.push(name.to_owned());
                    }
                }
            }
        }
        ItemKind::Agent => {
            for dir in &config.agent_dirs {
                let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "md")
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
mod tests {
    use super::*;
    use crate::env::FakeOs;
    use crate::manifest::{MANIFEST_SCHEMA, SourceDecl};

    fn manifest_with(name: &str, decl: SourceDecl) -> Manifest {
        let mut manifest = Manifest {
            schema: MANIFEST_SCHEMA,
            ..Manifest::default()
        };
        manifest.sources.insert(name.to_owned(), decl);
        manifest
    }

    #[test]
    fn remote_without_cache_is_pending_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let manifest = manifest_with(
            "vstack",
            SourceDecl {
                repo: Some("vanillagreencom/vstack".into()),
                path: None,
                enabled: true,
            },
        );
        let state = resolve(&env, &Scope::Global, "vstack", &manifest).unwrap();
        assert!(matches!(state, SourceState::Pending { .. }));
        assert!(matches!(
            require_ready(&env, &Scope::Global, "vstack", &manifest),
            Err(CoreError::SourcePending { .. })
        ));
    }

    #[test]
    fn path_sources_resolve_relative_to_scope_root() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(project.join("catalog/skills/gh")).unwrap();
        std::fs::write(project.join("catalog/skills/gh/SKILL.md"), "x").unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let manifest = manifest_with(
            "cat",
            SourceDecl {
                repo: None,
                path: Some("catalog".into()),
                enabled: true,
            },
        );
        let scope = Scope::Project {
            root: project.clone(),
        };
        let source = require_ready(&env, &scope, "cat", &manifest).unwrap();
        assert_eq!(source.root, project.join("catalog").canonicalize().unwrap());

        let config = source_config(&source.root).unwrap();
        assert_eq!(
            find_item(&source.root, &config, ItemKind::Skill, "gh"),
            Some(source.root.join("skills/gh"))
        );
        assert_eq!(list_items(&source.root, &config, ItemKind::Skill), ["gh"]);
    }

    #[test]
    fn disabled_and_missing_and_unknown_sources_are_distinct() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let mut manifest = manifest_with(
            "off",
            SourceDecl {
                repo: Some("a/b".into()),
                path: None,
                enabled: false,
            },
        );
        manifest.sources.insert(
            "gone".into(),
            SourceDecl {
                repo: None,
                path: Some("nowhere".into()),
                enabled: true,
            },
        );
        assert!(matches!(
            resolve(&env, &Scope::Global, "off", &manifest).unwrap(),
            SourceState::Disabled { .. }
        ));
        assert!(matches!(
            resolve(&env, &Scope::Global, "gone", &manifest).unwrap(),
            SourceState::Missing { .. }
        ));
        assert!(matches!(
            resolve(&env, &Scope::Global, "nope", &manifest),
            Err(CoreError::UnknownSource { .. })
        ));
    }

    #[test]
    fn phase_three_kinds_live_at_fixed_catalog_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let (root, config) = (tmp.path(), SourceConfig::default());
        for (kind, rel) in [
            (ItemKind::Hook, "hooks/guard.sh"),
            (ItemKind::Command, "commands/ship.md"),
            (ItemKind::McpServer, "mcp/gh.toml"),
        ] {
            assert_eq!(find_item(root, &config, kind, "guard"), None);
            let path = root.join(rel);
            std::fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
            std::fs::write(&path, "x").unwrap();
            let name = path.file_stem().unwrap().to_string_lossy();
            assert_eq!(find_item(root, &config, kind, &name), Some(path));
        }
    }

    #[test]
    fn v1_catalog_tables_parse_leniently() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("vstack.toml"),
            r#"
is_source_catalog = true
[catalog]
skills = ["skills", "extra-skills"]
[agent-skills]
rust = ["clippy"]
[role-skills]
engineer = ["dev"]
"#,
        )
        .unwrap();
        let config = source_config(tmp.path()).unwrap();
        assert_eq!(config.skill_dirs, ["skills", "extra-skills"]);
        assert_eq!(config.agent_skills["rust"], ["clippy"]);
        assert_eq!(config.role_skills["engineer"], ["dev"]);
    }
}
