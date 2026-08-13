use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::model::{DetectedHarness, HarnessId, ItemKind};

pub mod claude;
pub mod codex;
pub mod cursor;
pub mod opencode;
pub mod pi;

mod caps;
pub mod models;
pub use caps::{KindCaps, OpSupport, capabilities};

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

pub fn all_adapters() -> [&'static dyn HarnessAdapter; 5] {
    [
        &claude::Claude,
        &codex::Codex,
        &opencode::Opencode,
        &cursor::Cursor,
        &pi::Pi,
    ]
}

pub fn adapter(id: HarnessId) -> &'static dyn HarnessAdapter {
    match id {
        HarnessId::Claude => &claude::Claude,
        HarnessId::Codex => &codex::Codex,
        HarnessId::Opencode => &opencode::Opencode,
        HarnessId::Cursor => &cursor::Cursor,
        HarnessId::Pi => &pi::Pi,
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

    /// Nothing may be mutable where it cannot even be observed.
    #[test]
    fn no_capability_exceeds_observation() {
        for harness in HarnessId::ALL {
            for kind in ItemKind::ALL {
                let c = capabilities(harness, kind);
                for (op, sup) in [
                    ("adopt", c.adopt),
                    ("install", c.install),
                    ("toggle", c.toggle),
                    ("remove", c.remove),
                    ("refresh", c.refresh),
                ] {
                    assert!(
                        (!sup.project || c.observe.project) && (!sup.global || c.observe.global),
                        "{}/{}: {op} exceeds observe",
                        harness.name(),
                        kind.name(),
                    );
                }
            }
        }
    }
}
