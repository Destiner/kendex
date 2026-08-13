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

/// Current manifest schema. Schema 1 (v0.1) still loads; the first apply
/// upgrades it in place through the normal journaled plan. A schema newer
/// than this build refuses to load — downgrades must never corrupt.
pub const MANIFEST_SCHEMA: u32 = 2;
pub const OLDEST_READABLE_SCHEMA: u32 = 1;
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
    /// Which revision of a remote to read. A full commit id is a pin: that
    /// commit and no other, forever, and it works offline once cached. A
    /// tag or branch tracks — every refresh re-resolves it and the new
    /// content is previewed before anything is written. Absent tracks the
    /// repository's default branch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
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
    /// Allow-only tool intent: replaces a source-side `tools:` allowlist for
    /// this harness. Distinct from `deny_tools`, which only narrows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_tools: Option<Vec<String>>,
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

/// One declared plugin. The harness is part of the declaration because more
/// than one tool reads an `enabledPlugins` map of its own: without it, a
/// plugin meant for one tool would be switched on in every tool's settings,
/// which is a claim about software the user never installed there.
///
/// A declaration written before the harness was part of it belongs to Claude
/// Code — the only tool whose plugin switch vstack ever wrote — so that is
/// what an older manifest reads back as, and the next write records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct PluginDecl {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_plugin_harness")]
    pub harness: HarnessId,
}

fn default_plugin_harness() -> HarnessId {
    HarnessId::Claude
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
    /// Installed bundles: a curated set the catalog offers under one name.
    /// What the set holds is the catalog's to say and derives on every plan;
    /// this records only that the set is installed, and how its members
    /// install — the same choices any declaration makes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bundles: BTreeMap<String, ItemDecl>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pi_extensions: BTreeMap<String, ItemDecl>,
    /// Items the user removed and wants kept removed, by kind: a dependency
    /// another item requires, or a member of an installed bundle. A refresh
    /// honors these instead of re-deriving what was taken away, and the item
    /// that wanted them says so in the audit.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub suppressed: BTreeMap<crate::model::ItemKind, Vec<String>>,
    /// Optional dependencies taken at install time, per item that offers
    /// them. A choice, so it belongs here and survives refresh, cache loss,
    /// and other machines; what those choices pull in does not.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        rename = "optional-dependencies"
    )]
    pub optional_dependencies: BTreeMap<String, Vec<String>>,
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

    pub fn is_suppressed(&self, kind: crate::model::ItemKind, name: &str) -> bool {
        self.suppressed
            .get(&kind)
            .is_some_and(|names| names.iter().any(|held| held == name))
    }

    /// Record that this item stays removed. Re-suppressing is a no-op, so a
    /// second removal of the same name writes nothing new.
    pub fn suppress(&mut self, kind: crate::model::ItemKind, name: &str) {
        if self.is_suppressed(kind, name) {
            return;
        }
        let names = self.suppressed.entry(kind).or_default();
        names.push(name.to_owned());
        names.sort();
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
    if let Some(schema) = table.get("schema").and_then(toml::Value::as_integer)
        && schema > i64::from(MANIFEST_SCHEMA)
    {
        return Err(CoreError::SchemaTooNew {
            path: path.to_path_buf(),
            found: schema,
        });
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
/// Whatever schema was read, a mutation writes the current one — every
/// write path upgrades as a side effect of writing at all.
pub fn load_for_mutation(path: &Path) -> Result<Option<Manifest>> {
    match load(path)? {
        ManifestFile::Absent => Ok(None),
        ManifestFile::Legacy { .. } => Err(CoreError::LegacyManifest {
            path: path.to_path_buf(),
        }),
        ManifestFile::Current(mut manifest) => {
            manifest.schema = MANIFEST_SCHEMA;
            Ok(Some(*manifest))
        }
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
            rev: None,
            enabled: true,
        },
    );
    manifest.install.harnesses = detected_harnesses.to_vec();
    manifest
}

#[cfg(test)]
mod tests;
