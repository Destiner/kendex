use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessId {
    Claude,
    Codex,
    Opencode,
    Cursor,
    Pi,
}

impl HarnessId {
    pub const ALL: [HarnessId; 5] = [
        HarnessId::Claude,
        HarnessId::Codex,
        HarnessId::Opencode,
        HarnessId::Cursor,
        HarnessId::Pi,
    ];

    pub fn name(self) -> &'static str {
        match self {
            HarnessId::Claude => "claude",
            HarnessId::Codex => "codex",
            HarnessId::Opencode => "opencode",
            HarnessId::Cursor => "cursor",
            HarnessId::Pi => "pi",
        }
    }

    /// The product name people read — plan previews and drift details use
    /// this, never the internal id.
    pub fn display_name(self) -> &'static str {
        match self {
            HarnessId::Claude => "Claude Code",
            HarnessId::Codex => "Codex",
            HarnessId::Opencode => "OpenCode",
            HarnessId::Cursor => "Cursor",
            HarnessId::Pi => "Pi",
        }
    }

    /// v1 harness ids, including the `claude-code` long form.
    pub fn parse(value: &str) -> Option<HarnessId> {
        match value {
            "claude" | "claude-code" => Some(HarnessId::Claude),
            "codex" => Some(HarnessId::Codex),
            "opencode" => Some(HarnessId::Opencode),
            "cursor" => Some(HarnessId::Cursor),
            "pi" => Some(HarnessId::Pi),
            _ => None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKind {
    Agent,
    Skill,
    Hook,
    Command,
    McpServer,
    Plugin,
    PiExtension,
}

impl ItemKind {
    pub const ALL: [ItemKind; 7] = [
        ItemKind::Agent,
        ItemKind::Skill,
        ItemKind::Hook,
        ItemKind::Command,
        ItemKind::McpServer,
        ItemKind::Plugin,
        ItemKind::PiExtension,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Agent => "agent",
            ItemKind::Skill => "skill",
            ItemKind::Hook => "hook",
            ItemKind::Command => "command",
            ItemKind::McpServer => "mcp-server",
            ItemKind::Plugin => "plugin",
            ItemKind::PiExtension => "pi-extension",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(tag = "scope", rename_all = "kebab-case")]
pub enum Scope {
    Global,
    Project { root: PathBuf },
}

impl Scope {
    pub fn label(&self) -> String {
        match self {
            Scope::Global => "global".to_owned(),
            Scope::Project { root } => root.display().to_string(),
        }
    }
}

/// How an observed item exists on disk. Kinds that live as entries inside a
/// shared config file (MCP servers, some hooks) are `ConfigEntry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum FileState {
    File,
    Dir,
    Symlink { target: PathBuf, broken: bool },
    ConfigEntry,
}

/// One item as the scanner found it — read-only truth, no interpretation of
/// whether it is declared or managed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ObservedItem {
    pub kind: ItemKind,
    pub name: String,
    pub harness: HarnessId,
    pub scope: Scope,
    /// Path of the artifact, or of the config file that contains the entry.
    pub path: PathBuf,
    pub file_state: FileState,
    /// Observable enabled/disabled state; `None` when the harness has no
    /// observable toggle for this kind.
    pub enabled: Option<bool>,
    /// Best-effort provenance: git origin URL of the content's real location.
    pub origin: Option<String>,
    pub description: Option<String>,
}

/// A harness found on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectedHarness {
    pub harness: HarnessId,
    /// The directory whose existence marks the harness as installed.
    pub root: PathBuf,
    pub version: Option<String>,
}
