use std::path::{Path, PathBuf};

use super::{HarnessAdapter, ProjectMarker, Reader, Surface};
use crate::env::Env;
use crate::model::{HarnessId, ItemKind};

pub struct Copilot;

/// Copilot claims its own namespace and nothing else. It genuinely reads
/// `.claude/` and `.agents/` files too, but those belong to the harnesses
/// they are named for — claiming them would count one file on disk as two
/// installations (matrix §R6).
impl HarnessAdapter for Copilot {
    fn id(&self) -> HarnessId {
        HarnessId::Copilot
    }

    /// `COPILOT_HOME` relocates the whole config root; hardcoding the home
    /// directory scans the wrong machine state for anyone who sets it
    /// (matrix §3, §R4).
    fn default_global_root(&self, env: &Env) -> PathBuf {
        match env.var("COPILOT_HOME") {
            Some(home) => PathBuf::from(home),
            None => env.home.join(".copilot"),
        }
    }

    /// `.github/` alone marks nearly every repository, so only the files and
    /// directories Copilot itself creates count (matrix §3).
    fn project_markers(&self) -> &'static [ProjectMarker] {
        &[
            ProjectMarker::File(".github/copilot-instructions.md"),
            ProjectMarker::Dir(".github/agents"),
            ProjectMarker::Dir(".github/skills"),
            ProjectMarker::Dir(".github/hooks"),
        ]
    }

    fn global_surfaces(&self, kind: ItemKind, root: &Path, _env: &Env) -> Vec<Surface> {
        match kind {
            ItemKind::Agent => vec![Surface::files(root.join("agents"), &["agent.md"])],
            ItemKind::Skill => vec![Surface::SubdirPerItem {
                dir: root.join("skills"),
                marker: "SKILL.md",
            }],
            ItemKind::McpServer => vec![Surface::Structured {
                path: root.join("mcp-config.json"),
                reader: Reader::McpServersJson,
            }],
            // Commands have no Copilot surface at all (matrix §D8). Hook
            // files and the `enabledPlugins` map both need readers of their
            // own, so neither is claimed yet (matrix §7).
            ItemKind::Command | ItemKind::Hook | ItemKind::Plugin | ItemKind::PiExtension => {
                vec![]
            }
        }
    }

    fn project_surfaces(&self, kind: ItemKind, project: &Path, env: &Env) -> Vec<Surface> {
        let github = project.join(".github");
        match kind {
            ItemKind::McpServer => vec![Surface::Structured {
                path: github.join("mcp.json"),
                reader: Reader::McpServersJson,
            }],
            other => self.global_surfaces(other, &github, env),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    #[test]
    fn copilot_home_var_relocates_the_root() {
        let env = Env::fake("/h", FakeOs::Linux);
        assert_eq!(
            Copilot.default_global_root(&env),
            PathBuf::from("/h/.copilot")
        );

        let env = env.with_var("COPILOT_HOME", "/elsewhere/copilot");
        assert_eq!(
            Copilot.default_global_root(&env),
            PathBuf::from("/elsewhere/copilot")
        );
    }

    #[test]
    fn agents_carry_the_double_extension_and_projects_live_under_github() {
        for os in [FakeOs::Linux, FakeOs::Mac, FakeOs::Windows] {
            let env = Env::fake("/h", os);
            let root = Copilot.default_global_root(&env);
            assert_eq!(
                Copilot.global_surfaces(ItemKind::Agent, &root, &env),
                [Surface::files(
                    PathBuf::from("/h/.copilot/agents"),
                    &["agent.md"]
                )]
            );
            assert_eq!(
                Copilot.project_surfaces(ItemKind::Agent, Path::new("/p"), &env),
                [Surface::files(
                    PathBuf::from("/p/.github/agents"),
                    &["agent.md"]
                )]
            );
        }
    }

    #[test]
    fn each_scope_reads_its_own_mcp_file() {
        let env = Env::fake("/h", FakeOs::Linux);
        let root = Copilot.default_global_root(&env);
        assert_eq!(
            Copilot.global_surfaces(ItemKind::McpServer, &root, &env),
            [Surface::Structured {
                path: PathBuf::from("/h/.copilot/mcp-config.json"),
                reader: Reader::McpServersJson,
            }]
        );
        assert_eq!(
            Copilot.project_surfaces(ItemKind::McpServer, Path::new("/p"), &env),
            [Surface::Structured {
                path: PathBuf::from("/p/.github/mcp.json"),
                reader: Reader::McpServersJson,
            }]
        );
    }
}
