use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::model::{DetectedHarness, HarnessId, ItemKind};

pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod gemini;
pub mod opencode;
pub mod pi;

mod caps;
pub mod models;
pub use caps::{
    Enforcement, FormatCaps, KindCaps, McpTransport, NameRule, OpSupport, ToggleDirection,
    capabilities, format_caps, installable,
};

/// What marks a directory as a project for this harness during discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectMarker {
    Dir(&'static str),
    File(&'static str),
}

/// A place the scanner reads one kind from, plus how items are stored there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Surface {
    /// `<dir>/<name>.<ext>` — one item per file, one folder level of
    /// namespacing included (`ns/name.md` → item `ns/name`). A `.disabled`
    /// suffix on the full filename marks a disabled item. `prefix` restricts
    /// to filenames starting with it (opencode hook instructions).
    FileDir {
        dir: PathBuf,
        exts: &'static [&'static str],
        prefix: Option<&'static str>,
    },
    /// `<dir>/<name>/<marker>` — one item per subdirectory holding the
    /// marker file (`<marker>.disabled` marks a disabled item).
    SubdirPerItem { dir: PathBuf, marker: &'static str },
    /// Items are entries inside a structured file or tree; the reader names
    /// the harness-specific format the scanner must parse.
    Structured { path: PathBuf, reader: Reader },
}

impl Surface {
    pub fn files(dir: PathBuf, exts: &'static [&'static str]) -> Surface {
        Surface::FileDir {
            dir,
            exts,
            prefix: None,
        }
    }
}

/// Harness-specific structured formats. One variant per real on-disk format;
/// the scanner owns the parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reader {
    /// `{"mcpServers": {...}}` — claude `.mcp.json`, cursor `mcp.json`
    McpServersJson,
    /// `~/.claude.json` top-level `mcpServers`
    ClaudeUserMcp,
    /// `~/.claude.json` `projects.<root>.mcpServers`
    ClaudeUserProjectMcp { project: PathBuf },
    /// codex `config.toml` `[mcp_servers.<name>]`
    McpServersToml,
    /// opencode config `mcp` key — jsonc tolerated, per-entry `enabled`
    OpencodeMcp,
    /// opencode config `plugin` array — npm plugin refs
    OpencodePluginRefs,
    /// `{"hooks": {"<Event>": [{matcher?, hooks: [{command}]} | {command}]}}`
    /// — claude settings.json, codex/cursor hooks.json
    HooksObject,
    /// `~/.claude/plugins/installed_plugins.json` joined with settings
    /// `enabledPlugins`
    ClaudePluginRegistry,
    /// project `.claude/settings.json` + `.claude/settings.local.json`
    /// `enabledPlugins` entries
    ClaudeSettingsPlugins,
    /// `~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/` tree with
    /// `.codex-plugin/plugin.json`, toggles in config.toml `[plugins]`
    CodexPluginCache,
    /// `~/.cursor/plugins/{local,cache}` tree with `.cursor-plugin/plugin.json`
    CursorPluginDirs,
    /// pi `settings.json` `packages[]` entries
    PiPackages,
}

pub trait HarnessAdapter: Send + Sync {
    fn id(&self) -> HarnessId;

    /// Where the harness keeps global state when no settings override is set.
    fn default_global_root(&self, env: &Env) -> PathBuf;

    fn detect(&self, env: &Env, global_root: &Path) -> Option<DetectedHarness> {
        let _ = env;
        global_root.is_dir().then(|| DetectedHarness {
            harness: self.id(),
            root: global_root.to_path_buf(),
            version: None,
        })
    }

    fn project_markers(&self) -> &'static [ProjectMarker];

    /// Every read surface for `kind` at global scope. Empty = unsupported.
    fn global_surfaces(&self, kind: ItemKind, root: &Path, env: &Env) -> Vec<Surface>;

    /// Every read surface for `kind` inside a project. Empty = unsupported.
    fn project_surfaces(&self, kind: ItemKind, project: &Path, env: &Env) -> Vec<Surface>;
}

pub fn all_adapters() -> [&'static dyn HarnessAdapter; 7] {
    [
        &claude::Claude,
        &codex::Codex,
        &opencode::Opencode,
        &cursor::Cursor,
        &pi::Pi,
        &gemini::Gemini,
        &copilot::Copilot,
    ]
}

pub fn adapter(id: HarnessId) -> &'static dyn HarnessAdapter {
    match id {
        HarnessId::Claude => &claude::Claude,
        HarnessId::Codex => &codex::Codex,
        HarnessId::Opencode => &opencode::Opencode,
        HarnessId::Cursor => &cursor::Cursor,
        HarnessId::Pi => &pi::Pi,
        HarnessId::Gemini => &gemini::Gemini,
        HarnessId::Copilot => &copilot::Copilot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    #[test]
    fn adapter_registry_is_complete_and_ordered() {
        let ids: Vec<_> = all_adapters().iter().map(|a| a.id()).collect();
        assert_eq!(ids, HarnessId::ALL);
        for id in HarnessId::ALL {
            assert_eq!(adapter(id).id(), id);
        }
    }

    /// The capability table's observe column must mirror what the adapters
    /// actually declare — UI gating and scan behavior cannot drift apart.
    #[test]
    fn observe_capabilities_match_declared_surfaces() {
        let env = Env::fake("/home/user", FakeOs::Linux);
        let project = Path::new("/home/user/dev/proj");
        for a in all_adapters() {
            let root = a.default_global_root(&env);
            for kind in ItemKind::ALL {
                let caps = capabilities(a.id(), kind);
                assert_eq!(
                    caps.observe.global,
                    !a.global_surfaces(kind, &root, &env).is_empty(),
                    "{}/{} global observe",
                    a.id().name(),
                    kind.name(),
                );
                assert_eq!(
                    caps.observe.project,
                    !a.project_surfaces(kind, project, &env).is_empty(),
                    "{}/{} project observe",
                    a.id().name(),
                    kind.name(),
                );
            }
        }
    }

    /// One kind stored as another is the whole of the cross-kind mapping:
    /// a renderer exists for exactly this pair, so a new entry in the
    /// table must arrive with the renderer that serves it.
    #[test]
    fn the_only_kind_stored_as_another_is_a_codex_command() {
        for harness in HarnessId::ALL {
            for kind in ItemKind::ALL {
                if let Some(emitted) = capabilities(harness, kind).installs_as {
                    assert_eq!(
                        (harness, kind, emitted),
                        (HarnessId::Codex, ItemKind::Command, ItemKind::Skill)
                    );
                }
            }
        }
    }

    /// A hook the tool merely reads must never be presented as one it runs.
    /// Every harness with a hook surface says which it is, and the harnesses
    /// without one are exactly the rows that say nothing.
    #[test]
    fn every_hook_row_says_whether_the_tool_runs_it() {
        for harness in HarnessId::ALL {
            let hook = capabilities(harness, ItemKind::Hook);
            let observed = hook.observe.project || hook.observe.global;
            assert_eq!(
                hook.enforcement == Enforcement::NotApplicable,
                !observed,
                "{} hook enforcement",
                harness.name(),
            );
            for kind in ItemKind::ALL.into_iter().filter(|k| *k != ItemKind::Hook) {
                assert_eq!(
                    capabilities(harness, kind).enforcement,
                    Enforcement::NotApplicable,
                    "{}/{} claims enforcement",
                    harness.name(),
                    kind.name(),
                );
            }
        }
    }

    /// The transport list and the MCP row describe one fact from two sides:
    /// a harness that reads no servers has no way to reach one.
    #[test]
    fn mcp_transports_agree_with_the_mcp_row() {
        for harness in HarnessId::ALL {
            let mcp = capabilities(harness, ItemKind::McpServer);
            assert_eq!(
                format_caps(harness).mcp_transports.is_empty(),
                mcp.observe == caps::NONE,
                "{} mcp transports",
                harness.name(),
            );
        }
    }

    /// Copilot's repository settings merge as a union: a repo file adds to
    /// `disabledSkills` and `disabledMcpServers` but cannot take a name off
    /// them, so the switch there only turns things off (matrix §R7).
    #[test]
    fn copilot_skills_and_servers_switch_off_only() {
        for kind in [ItemKind::Skill, ItemKind::McpServer] {
            assert_eq!(
                capabilities(HarnessId::Copilot, kind).toggle_direction,
                ToggleDirection::DisableOnly,
            );
        }
        assert_eq!(
            capabilities(HarnessId::Claude, ItemKind::Skill).toggle_direction,
            ToggleDirection::Both,
        );
    }

    /// Gemini and Copilot are observed, never written: the management verbs
    /// arrive with the adapters that implement them, and until then neither
    /// is offered anywhere an install target is chosen.
    #[test]
    fn the_new_harnesses_only_read() {
        let install_targets: Vec<_> = HarnessId::ALL
            .into_iter()
            .filter(|h| installable(*h))
            .collect();
        assert_eq!(
            install_targets,
            [
                HarnessId::Claude,
                HarnessId::Codex,
                HarnessId::Opencode,
                HarnessId::Cursor,
                HarnessId::Pi,
            ]
        );
        for harness in [HarnessId::Gemini, HarnessId::Copilot] {
            for kind in ItemKind::ALL {
                let c = capabilities(harness, kind);
                for (op, support) in [
                    ("adopt", c.adopt),
                    ("install", c.install),
                    ("toggle", c.toggle),
                    ("remove", c.remove),
                    ("refresh", c.refresh),
                ] {
                    assert_eq!(
                        support,
                        caps::NONE,
                        "{}/{} {op}",
                        harness.name(),
                        kind.name(),
                    );
                }
            }
        }
    }

    /// Nothing may be mutable where what it writes cannot be observed. A
    /// kind the harness stores as another one is checked against that
    /// kind's surfaces, because that is where its artifact lands.
    #[test]
    fn no_capability_exceeds_observation() {
        for harness in HarnessId::ALL {
            for kind in ItemKind::ALL {
                let c = capabilities(harness, kind);
                let written = match c.installs_as {
                    Some(emitted) => capabilities(harness, emitted).observe,
                    None => c.observe,
                };
                for (op, sup, observe) in [
                    ("adopt", c.adopt, c.observe),
                    ("install", c.install, written),
                    ("toggle", c.toggle, written),
                    ("remove", c.remove, written),
                    ("refresh", c.refresh, written),
                ] {
                    assert!(
                        (!sup.project || observe.project) && (!sup.global || observe.global),
                        "{}/{}: {op} exceeds observe",
                        harness.name(),
                        kind.name(),
                    );
                }
            }
        }
    }
}
