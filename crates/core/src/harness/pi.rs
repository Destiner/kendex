use std::path::{Path, PathBuf};

use super::{HarnessAdapter, ProjectMarker, Reader, Surface};
use crate::env::Env;
use crate::model::{HarnessId, ItemKind};

pub struct Pi;

const EXTENSION_EXTS: &[&str] = &["ts", "js"];

impl HarnessAdapter for Pi {
    fn id(&self) -> HarnessId {
        HarnessId::Pi
    }

    fn default_global_root(&self, env: &Env) -> PathBuf {
        match env.var("PI_CODING_AGENT_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => env.home.join(".pi/agent"),
        }
    }

    fn project_markers(&self) -> &'static [ProjectMarker] {
        &[ProjectMarker::Dir(".pi"), ProjectMarker::Dir(".agents")]
    }

    fn global_surfaces(&self, kind: ItemKind, root: &Path, _env: &Env) -> Vec<Surface> {
        match kind {
            ItemKind::Agent => vec![Surface::files(root.join("agents"), &["md"])],
            ItemKind::Skill => vec![Surface::SubdirPerItem {
                dir: root.join("skills"),
                marker: "SKILL.md",
            }],
            // Hooks ride the pi-hooks carrier: the registry vstack renders
            // is what the carrier's listeners execute. pi has no MCP.
            ItemKind::Hook => vec![Surface::Structured {
                path: root.join("hooks.json"),
                reader: Reader::HooksObject,
            }],
            ItemKind::McpServer | ItemKind::Plugin => vec![],
            ItemKind::Command => vec![Surface::files(root.join("prompts"), &["md"])],
            ItemKind::PiExtension => vec![
                Surface::Structured {
                    path: root.join("settings.json"),
                    reader: Reader::PiPackages,
                },
                Surface::FileDir {
                    dir: root.join("extensions"),
                    exts: EXTENSION_EXTS,
                    prefix: None,
                },
            ],
        }
    }

    fn project_surfaces(&self, kind: ItemKind, project: &Path, _env: &Env) -> Vec<Surface> {
        let dot = project.join(".pi");
        match kind {
            ItemKind::Agent => vec![Surface::files(dot.join("agents"), &["md"])],
            // Shared physical target with codex — scan dedupe couples them.
            ItemKind::Skill => vec![Surface::SubdirPerItem {
                dir: project.join(".agents/skills"),
                marker: "SKILL.md",
            }],
            ItemKind::Hook => vec![Surface::Structured {
                path: dot.join("hooks.json"),
                reader: Reader::HooksObject,
            }],
            ItemKind::McpServer | ItemKind::Plugin => vec![],
            ItemKind::Command => vec![Surface::files(dot.join("prompts"), &["md"])],
            ItemKind::PiExtension => vec![
                Surface::Structured {
                    path: dot.join("settings.json"),
                    reader: Reader::PiPackages,
                },
                Surface::FileDir {
                    dir: dot.join("extensions"),
                    exts: EXTENSION_EXTS,
                    prefix: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    #[test]
    fn agent_dir_var_relocates_the_root() {
        let env = Env::fake("/h", FakeOs::Linux);
        assert_eq!(Pi.default_global_root(&env), PathBuf::from("/h/.pi/agent"));

        let env = env.with_var("PI_CODING_AGENT_DIR", "/pi-root");
        assert_eq!(Pi.default_global_root(&env), PathBuf::from("/pi-root"));
    }

    #[test]
    fn skills_share_the_codex_tree_and_packages_live_in_settings() {
        for os in [FakeOs::Linux, FakeOs::Mac, FakeOs::Windows] {
            let env = Env::fake("/h", os);
            assert_eq!(
                Pi.project_surfaces(ItemKind::Skill, Path::new("/p"), &env),
                [Surface::SubdirPerItem {
                    dir: PathBuf::from("/p/.agents/skills"),
                    marker: "SKILL.md",
                }]
            );
            assert_eq!(
                Pi.project_surfaces(ItemKind::PiExtension, Path::new("/p"), &env),
                [
                    Surface::Structured {
                        path: PathBuf::from("/p/.pi/settings.json"),
                        reader: Reader::PiPackages,
                    },
                    Surface::FileDir {
                        dir: PathBuf::from("/p/.pi/extensions"),
                        exts: EXTENSION_EXTS,
                        prefix: None,
                    },
                ]
            );
        }
    }
}
