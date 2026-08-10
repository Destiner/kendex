use std::path::{Path, PathBuf};

use super::{HarnessAdapter, ProjectMarker, Reader, Surface};
use crate::env::Env;
use crate::model::{DetectedHarness, HarnessId, ItemKind};

pub struct Opencode;

const PLUGIN_EXTS: &[&str] = &["js", "ts", "mjs", "cjs"];

fn global_config_file(root: &Path, env: &Env) -> PathBuf {
    if let Some(explicit) = env.var("OPENCODE_CONFIG") {
        return PathBuf::from(explicit);
    }
    let jsonc = root.join("opencode.jsonc");
    if jsonc.is_file() {
        jsonc
    } else {
        root.join("opencode.json")
    }
}

fn project_config_file(project: &Path) -> PathBuf {
    let json = project.join("opencode.json");
    let jsonc = project.join("opencode.jsonc");
    if !json.is_file() && jsonc.is_file() {
        jsonc
    } else {
        json
    }
}

impl HarnessAdapter for Opencode {
    fn id(&self) -> HarnessId {
        HarnessId::Opencode
    }

    fn default_global_root(&self, env: &Env) -> PathBuf {
        if let Some(config) = env.var("OPENCODE_CONFIG")
            && let Some(parent) = Path::new(config).parent()
        {
            return parent.to_path_buf();
        }
        if let Some(dir) = env.var("OPENCODE_CONFIG_DIR") {
            return PathBuf::from(dir);
        }
        env.home.join(".config/opencode")
    }

    fn detect(&self, env: &Env, global_root: &Path) -> Option<DetectedHarness> {
        (global_root.is_dir() || global_config_file(global_root, env).is_file()).then(|| {
            DetectedHarness {
                harness: HarnessId::Opencode,
                root: global_root.to_path_buf(),
                version: None,
            }
        })
    }

    fn project_markers(&self) -> &'static [ProjectMarker] {
        &[
            ProjectMarker::Dir(".opencode"),
            ProjectMarker::File("opencode.json"),
            ProjectMarker::File("opencode.jsonc"),
        ]
    }

    fn global_surfaces(&self, kind: ItemKind, root: &Path, env: &Env) -> Vec<Surface> {
        let config = global_config_file(root, env);
        surfaces(kind, root, config)
    }

    fn project_surfaces(&self, kind: ItemKind, project: &Path, _env: &Env) -> Vec<Surface> {
        let config = project_config_file(project);
        surfaces(kind, &project.join(".opencode"), config)
    }
}

fn surfaces(kind: ItemKind, base: &Path, config: PathBuf) -> Vec<Surface> {
    match kind {
        ItemKind::Agent => vec![Surface::files(base.join("agents"), &["md"])],
        ItemKind::Skill => vec![Surface::SubdirPerItem {
            dir: base.join("skills"),
            marker: "SKILL.md",
        }],
        // opencode has no native hook surface; only the instruction files the
        // engine itself renders are observable.
        ItemKind::Hook => vec![Surface::FileDir {
            dir: base.join("instructions"),
            exts: &["md"],
            prefix: Some("vstack-hook-"),
        }],
        ItemKind::Command => vec![
            Surface::files(base.join("commands"), &["md"]),
            Surface::files(base.join("command"), &["md"]),
        ],
        ItemKind::McpServer => vec![Surface::Structured {
            path: config,
            reader: Reader::OpencodeMcp,
        }],
        ItemKind::Plugin => vec![
            Surface::FileDir {
                dir: base.join("plugins"),
                exts: PLUGIN_EXTS,
                prefix: None,
            },
            Surface::Structured {
                path: config,
                reader: Reader::OpencodePluginRefs,
            },
        ],
        ItemKind::PiExtension => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    #[test]
    fn root_resolution_honors_env_overrides_in_order() {
        let env = Env::fake("/h", FakeOs::Linux);
        assert_eq!(
            Opencode.default_global_root(&env),
            PathBuf::from("/h/.config/opencode")
        );

        let with_dir = env.clone().with_var("OPENCODE_CONFIG_DIR", "/oc");
        assert_eq!(
            Opencode.default_global_root(&with_dir),
            PathBuf::from("/oc")
        );

        let with_config = with_dir.with_var("OPENCODE_CONFIG", "/explicit/opencode.json");
        assert_eq!(
            Opencode.default_global_root(&with_config),
            PathBuf::from("/explicit")
        );
    }

    #[test]
    fn commands_scan_both_plural_and_legacy_singular_dirs() {
        let env = Env::fake("/h", FakeOs::Linux);
        let found = Opencode.project_surfaces(ItemKind::Command, Path::new("/p"), &env);
        assert_eq!(
            found,
            [
                Surface::files(PathBuf::from("/p/.opencode/commands"), &["md"]),
                Surface::files(PathBuf::from("/p/.opencode/command"), &["md"]),
            ]
        );
    }

    #[test]
    fn missing_config_defaults_to_json_variant() {
        let env = Env::fake("/h", FakeOs::Linux);
        let mcp = Opencode.project_surfaces(ItemKind::McpServer, Path::new("/p"), &env);
        assert_eq!(
            mcp,
            [Surface::Structured {
                path: PathBuf::from("/p/opencode.json"),
                reader: Reader::OpencodeMcp,
            }]
        );
    }
}
