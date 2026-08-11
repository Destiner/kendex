use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::{atomic_write, read_if_exists};
use crate::model::{HarnessId, Scope};

mod validate;
pub use validate::{Finding, validate};

pub const MANIFEST_SCHEMA: u32 = 1;
pub const DEFAULT_SOURCE_NAME: &str = "vstack";
pub const DEFAULT_SOURCE_REPO: &str = "vanillagreencom/vstack";
/// The reserved source name for content adopted into this scope.
pub const LOCAL_SOURCE_NAME: &str = "local";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum Method {
    #[default]
    Symlink,
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct SourceDecl {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct InstallDefaults {
    #[serde(default)]
    pub harnesses: Vec<HarnessId>,
    #[serde(default)]
    pub method: Method,
}

/// One declared item: `[agents.<name>]` / `[skills.<name>]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct ItemDecl {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnesses: Option<Vec<HarnessId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<Method>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl ItemDecl {
    pub fn from_source(source: &str) -> ItemDecl {
        ItemDecl {
            source: source.to_owned(),
            harnesses: None,
            method: None,
            enabled: true,
        }
    }
}

/// Typed `[agent-frontmatter.<harness>.<agent>]` overrides — v1's field set.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct FrontmatterOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_subagents: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname_candidates: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct PluginDecl {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct CustomHook {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_hook_agents")]
    pub agents: HookAgents,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(untagged)]
pub enum HookAgents {
    /// `"all"`, a role name, or a single agent name.
    One(String),
    Many(Vec<String>),
}

fn default_hook_agents() -> HookAgents {
    HookAgents::One("all".to_owned())
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub schema: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, SourceDecl>,
    #[serde(default)]
    pub install: InstallDefaults,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agents: BTreeMap<String, ItemDecl>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skills: BTreeMap<String, ItemDecl>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hooks: BTreeMap<String, ItemDecl>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, ItemDecl>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        rename = "mcp-servers"
    )]
    pub mcp_servers: BTreeMap<String, ItemDecl>,
    /// Plugins are observe + enable/disable only; the key is
    /// `name@marketplace`, provenance lives in the lock.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub plugins: BTreeMap<String, PluginDecl>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pi_extensions: BTreeMap<String, ItemDecl>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_skills: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_launch_instructions: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_additional_instructions: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skill_instructions: BTreeMap<String, String>,
    /// `[agent-frontmatter.<harness>.<agent>]`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_frontmatter: BTreeMap<String, BTreeMap<String, FrontmatterOverrides>>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "custom-hooks"
    )]
    pub custom_hooks: Vec<CustomHook>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_skills_dir: Option<String>,
}

impl Manifest {
    pub fn declared(&self, kind: crate::model::ItemKind) -> &BTreeMap<String, ItemDecl> {
        static EMPTY: std::sync::LazyLock<BTreeMap<String, ItemDecl>> =
            std::sync::LazyLock::new(BTreeMap::new);
        match kind {
            crate::model::ItemKind::Agent => &self.agents,
            crate::model::ItemKind::Skill => &self.skills,
            crate::model::ItemKind::Hook => &self.hooks,
            crate::model::ItemKind::Command => &self.commands,
            crate::model::ItemKind::McpServer => &self.mcp_servers,
            crate::model::ItemKind::PiExtension => &self.pi_extensions,
            crate::model::ItemKind::Plugin => &EMPTY,
        }
    }

    pub fn declared_mut(
        &mut self,
        kind: crate::model::ItemKind,
    ) -> &mut BTreeMap<String, ItemDecl> {
        match kind {
            crate::model::ItemKind::Agent => &mut self.agents,
            crate::model::ItemKind::Skill => &mut self.skills,
            crate::model::ItemKind::Hook => &mut self.hooks,
            crate::model::ItemKind::Command => &mut self.commands,
            crate::model::ItemKind::McpServer => &mut self.mcp_servers,
            crate::model::ItemKind::PiExtension => &mut self.pi_extensions,
            crate::model::ItemKind::Plugin => {
                unreachable!("plugins declare through [plugins.<key>] with only an enabled flag")
            }
        }
    }
}

/// What sits at a manifest path. A schema-less file is a v1 manifest: v2
/// never mutates it — hard "migration required" error until the importer.
#[derive(Debug, Clone, PartialEq)]
pub enum ManifestFile {
    Absent,
    Legacy { raw: String },
    Current(Box<Manifest>),
}

pub fn manifest_path(env: &Env, scope: &Scope) -> std::path::PathBuf {
    match scope {
        Scope::Global => env.global_manifest_file(),
        Scope::Project { root } => Env::project_manifest_file(root),
    }
}

pub fn load(path: &Path) -> Result<ManifestFile> {
    let Some(text) = read_if_exists(path)? else {
        return Ok(ManifestFile::Absent);
    };
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| CoreError::TomlParse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
    if !table.contains_key("schema") {
        return Ok(ManifestFile::Legacy { raw: text });
    }
    let findings = validate(&table);
    if !findings.is_empty() {
        return Err(CoreError::ManifestInvalid {
            path: path.to_path_buf(),
            findings: findings.iter().map(Finding::to_string).collect(),
        });
    }
    let manifest: Manifest =
        toml::from_str(&text).map_err(|e: toml::de::Error| CoreError::TomlParse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
    Ok(ManifestFile::Current(Box::new(manifest)))
}

pub fn save(path: &Path, manifest: &Manifest) -> Result<()> {
    let text = toml::to_string_pretty(manifest).map_err(|e| CoreError::TomlParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    atomic_write(path, &text)
}

/// Load for mutation: a legacy file is a hard error, never a write target.
pub fn load_for_mutation(path: &Path) -> Result<Option<Manifest>> {
    match load(path)? {
        ManifestFile::Absent => Ok(None),
        ManifestFile::Legacy { .. } => Err(CoreError::LegacyManifest {
            path: path.to_path_buf(),
        }),
        ManifestFile::Current(manifest) => Ok(Some(*manifest)),
    }
}

/// First manifest for a scope: the default source is seeded exactly once,
/// here — later reconciliation never re-adds it (its removal is durable).
pub fn seed(detected_harnesses: &[HarnessId]) -> Manifest {
    let mut manifest = Manifest {
        schema: MANIFEST_SCHEMA,
        ..Manifest::default()
    };
    manifest.sources.insert(
        DEFAULT_SOURCE_NAME.to_owned(),
        SourceDecl {
            repo: Some(DEFAULT_SOURCE_REPO.to_owned()),
            path: None,
            enabled: true,
        },
    );
    manifest.install.harnesses = detected_harnesses.to_vec();
    manifest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ItemKind;

    #[test]
    fn round_trips_the_binding_skeleton() {
        let text = r#"
schema = 1

[sources.vstack]
repo = "vanillagreencom/vstack"
enabled = true

[install]
harnesses = ["claude", "pi"]
method = "symlink"

[agents.orch]
source = "vstack"

[skills.github]
source = "vstack"
method = "copy"
enabled = false

[agent-skills]
orch = ["github"]

[agent-frontmatter.claude.orch]
model = "opus"
deny-tools = ["WebSearch"]

[[custom-hooks]]
event = "PreToolUse"
matcher = "Bash"
command = "./guard.sh"

[skill-instructions]
github = "prefer gh cli"
"#;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("vstack.toml");
        std::fs::write(&path, text).unwrap();

        let ManifestFile::Current(manifest) = load(&path).unwrap() else {
            panic!("expected current manifest");
        };
        assert_eq!(
            manifest.sources["vstack"].repo.as_deref(),
            Some("vanillagreencom/vstack")
        );
        assert_eq!(
            manifest.install.harnesses,
            [HarnessId::Claude, HarnessId::Pi]
        );
        assert!(!manifest.skills["github"].enabled);
        assert_eq!(manifest.skills["github"].method, Some(Method::Copy));
        assert_eq!(
            manifest.agent_frontmatter["claude"]["orch"].deny_tools,
            Some(vec!["WebSearch".to_owned()])
        );
        assert_eq!(manifest.custom_hooks[0].event, "PreToolUse");

        save(&path, &manifest).unwrap();
        let ManifestFile::Current(reloaded) = load(&path).unwrap() else {
            panic!("expected current manifest after save");
        };
        assert_eq!(reloaded, manifest);
    }

    #[test]
    fn schema_less_file_is_legacy_and_never_a_mutation_target() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("vstack.toml");
        let v1 = "[agent-skills]\nrust = [\"clippy\"]\n";
        std::fs::write(&path, v1).unwrap();

        assert!(matches!(load(&path).unwrap(), ManifestFile::Legacy { .. }));
        assert!(matches!(
            load_for_mutation(&path),
            Err(CoreError::LegacyManifest { .. })
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), v1);
    }

    #[test]
    fn seed_declares_the_default_source_once() {
        let manifest = seed(&[HarnessId::Claude]);
        assert!(manifest.sources[DEFAULT_SOURCE_NAME].enabled);
        assert_eq!(manifest.declared(ItemKind::Agent).len(), 0);
        assert_eq!(manifest.install.harnesses, [HarnessId::Claude]);
    }
}
