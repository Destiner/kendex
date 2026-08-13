use std::path::{Path, PathBuf};

use super::{HarnessAdapter, ProjectMarker, Reader, Surface};
use crate::env::Env;
use crate::model::{HarnessId, ItemKind};

pub struct Gemini;

/// Both scopes hold the same layout under their own root, which is why the
/// surface lists below differ only in where they start (matrix §1).
fn surfaces(kind: ItemKind, root: &Path) -> Vec<Surface> {
    match kind {
        ItemKind::Agent => vec![Surface::files(root.join("agents"), &["md"])],
        ItemKind::Skill => vec![Surface::SubdirPerItem {
            dir: root.join("skills"),
            marker: "SKILL.md",
        }],
        // Only `.toml` loads; a subdirectory becomes a `:` namespace.
        ItemKind::Command => vec![Surface::files(root.join("commands"), &["toml"])],
        // Gemini's hook entries carry the same matcher-plus-handlers shape
        // claude's settings.json does (matrix §1).
        ItemKind::Hook => vec![Surface::Structured {
            path: root.join("settings.json"),
            reader: Reader::HooksObject,
        }],
        ItemKind::McpServer => vec![Surface::Structured {
            path: root.join("settings.json"),
            reader: Reader::McpServersJson,
        }],
        // Extensions are global-only, so the project list stays empty; the
        // caller decides which root reaches here (matrix §1, §R1).
        ItemKind::Plugin | ItemKind::PiExtension => vec![],
    }
}

impl HarnessAdapter for Gemini {
    fn id(&self) -> HarnessId {
        HarnessId::Gemini
    }

    /// No documented variable relocates this root — only the two system
    /// settings paths are overridable (matrix §3).
    fn default_global_root(&self, env: &Env) -> PathBuf {
        env.home.join(".gemini")
    }

    fn project_markers(&self) -> &'static [ProjectMarker] {
        &[
            ProjectMarker::Dir(".gemini"),
            ProjectMarker::File("GEMINI.md"),
        ]
    }

    fn global_surfaces(&self, kind: ItemKind, root: &Path, _env: &Env) -> Vec<Surface> {
        match kind {
            // An installed extension is a directory carrying its manifest.
            ItemKind::Plugin => vec![Surface::SubdirPerItem {
                dir: root.join("extensions"),
                marker: "gemini-extension.json",
            }],
            other => surfaces(other, root),
        }
    }

    fn project_surfaces(&self, kind: ItemKind, project: &Path, _env: &Env) -> Vec<Surface> {
        surfaces(kind, &project.join(".gemini"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    #[test]
    fn both_scopes_share_one_layout_under_their_own_root() {
        for os in [FakeOs::Linux, FakeOs::Mac, FakeOs::Windows] {
            let env = Env::fake("/h", os);
            let root = Gemini.default_global_root(&env);
            assert_eq!(root, PathBuf::from("/h/.gemini"));

            assert_eq!(
                Gemini.global_surfaces(ItemKind::Command, &root, &env),
                [Surface::files(
                    PathBuf::from("/h/.gemini/commands"),
                    &["toml"]
                )]
            );
            assert_eq!(
                Gemini.project_surfaces(ItemKind::Command, Path::new("/p"), &env),
                [Surface::files(
                    PathBuf::from("/p/.gemini/commands"),
                    &["toml"]
                )]
            );
            assert_eq!(
                Gemini.project_surfaces(ItemKind::McpServer, Path::new("/p"), &env),
                [Surface::Structured {
                    path: PathBuf::from("/p/.gemini/settings.json"),
                    reader: Reader::McpServersJson,
                }]
            );
        }
    }

    #[test]
    fn extensions_exist_at_global_scope_only() {
        let env = Env::fake("/h", FakeOs::Linux);
        let root = Gemini.default_global_root(&env);
        assert_eq!(
            Gemini.global_surfaces(ItemKind::Plugin, &root, &env),
            [Surface::SubdirPerItem {
                dir: PathBuf::from("/h/.gemini/extensions"),
                marker: "gemini-extension.json",
            }]
        );
        assert_eq!(
            Gemini.project_surfaces(ItemKind::Plugin, Path::new("/p"), &env),
            []
        );
    }
}
