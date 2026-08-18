//! A catalog's own configuration — `kendex.toml` (or the pre-rename
//! `kendex.toml`) plus a Claude plugin registry where one exists — and the
//! item lookups that read it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::error::Result;
use crate::model::ItemKind;
use crate::source_read::SealedSource;

use super::bundles::{self, CatalogBundle};
use super::catalog;
use super::plugin_registry::{self, CatalogFinding, Registry};

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
    // Catalogs are foreign repos we cannot rename: read the new file name
    // first and fall back to the old; a catalog carrying both is served
    // from the new one rather than refused — refusing would brick every
    // subscriber over a state only the catalog's author can fix.
    let text = match sealed.read_if_exists(&sealed.root().join(crate::rename::MANIFEST_FILE))? {
        Some(text) => Some(text),
        None => sealed.read_if_exists(&sealed.root().join(crate::rename::LEGACY_MANIFEST_FILE))?,
    };
    let Some(text) = text else {
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
