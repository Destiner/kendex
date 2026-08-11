use std::path::PathBuf;

use crate::env::Env;
use crate::harness::adapter;
use crate::model::{HarnessId, Scope};

/// Where one hook's artifacts live for a harness at a scope. Install and
/// removal both read this, so the command string they register and strip can
/// never disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum HookTarget {
    /// A shell script the harness runs, registered in a JSON hooks file.
    Script {
        path: PathBuf,
        command: String,
        registry: PathBuf,
        /// codex gates hooks behind `[features] hooks = true`.
        feature: Option<PathBuf>,
    },
    /// An instruction file the opencode config references — opencode has no
    /// native hook surface, so the constraint travels as prose.
    Instruction {
        path: PathBuf,
        config: PathBuf,
        reference: String,
    },
    /// A cursor advisory rule: a file, no registration.
    Rule { path: PathBuf },
}

pub(super) fn hook_target(
    env: &Env,
    scope: &Scope,
    harness: HarnessId,
    name: &str,
) -> Option<HookTarget> {
    match harness {
        HarnessId::Claude => {
            let (dir, registry) = match scope {
                Scope::Global => {
                    let root = adapter(harness).default_global_root(env);
                    (root.join("hooks"), root.join("settings.json"))
                }
                Scope::Project { root } => {
                    (root.join(".claude/hooks"), claude_settings(env, scope))
                }
            };
            let path = dir.join(format!("{name}.sh"));
            let command = match scope {
                Scope::Global => format!("bash {}", path.display()),
                Scope::Project { .. } => {
                    format!("bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/{name}.sh\"")
                }
            };
            Some(HookTarget::Script {
                path,
                command,
                registry,
                feature: None,
            })
        }
        HarnessId::Codex => {
            let root = match scope {
                Scope::Global => adapter(harness).default_global_root(env),
                Scope::Project { root } => root.join(".codex"),
            };
            let path = root.join("hooks").join(format!("{name}.sh"));
            let command = match scope {
                Scope::Global => format!("bash {}", path.display()),
                Scope::Project { .. } => {
                    format!("bash \"$(git rev-parse --show-toplevel)/.codex/hooks/{name}.sh\"")
                }
            };
            Some(HookTarget::Script {
                path,
                command,
                registry: root.join("hooks.json"),
                feature: Some(root.join("config.toml")),
            })
        }
        HarnessId::Opencode => {
            let file = format!("vstack-hook-{name}.md");
            let (base, reference) = match scope {
                Scope::Global => (
                    adapter(harness).default_global_root(env),
                    format!("instructions/{file}"),
                ),
                Scope::Project { root } => (
                    root.join(".opencode"),
                    format!(".opencode/instructions/{file}"),
                ),
            };
            Some(HookTarget::Instruction {
                path: base.join("instructions").join(&file),
                config: crate::harness::opencode::config_file(env, scope),
                reference,
            })
        }
        HarnessId::Cursor => match scope {
            Scope::Project { root } => Some(HookTarget::Rule {
                path: root
                    .join(".cursor/rules")
                    .join(format!("safety-{name}.mdc")),
            }),
            Scope::Global => None,
        },
        // pi hooks belong to the pi-hooks extension, not to files we manage.
        HarnessId::Pi => None,
    }
}

/// The settings file carrying claude's hook registrations and plugin toggles.
pub(super) fn claude_settings(env: &Env, scope: &Scope) -> PathBuf {
    match scope {
        Scope::Global => adapter(HarnessId::Claude)
            .default_global_root(env)
            .join("settings.json"),
        Scope::Project { root } => root.join(".claude/settings.json"),
    }
}

/// The file `mcpServers` entries are written to. Project servers belong to
/// the repo's `.mcp.json`; global ones to the user file.
pub(super) fn mcp_registry(env: &Env, scope: &Scope, harness: HarnessId) -> Option<PathBuf> {
    if harness != HarnessId::Claude {
        return None;
    }
    Some(match scope {
        Scope::Global => env.home.join(".claude.json"),
        Scope::Project { root } => root.join(".mcp.json"),
    })
}

pub(super) fn plugin_settings(env: &Env, scope: &Scope, harness: HarnessId) -> Option<PathBuf> {
    (harness == HarnessId::Claude).then(|| claude_settings(env, scope))
}

/// A declared-disabled artifact keeps its content under the `.disabled`
/// name; toggling is a rename (invariant 5).
pub(super) fn disabled_name(path: &std::path::Path) -> PathBuf {
    PathBuf::from(format!("{}.disabled", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    #[test]
    fn claude_hooks_use_the_project_dir_variable_and_absolute_global_paths() {
        let env = Env::fake("/h", FakeOs::Linux);
        let scope = Scope::Project {
            root: PathBuf::from("/p"),
        };
        let Some(HookTarget::Script {
            path,
            command,
            registry,
            feature,
        }) = hook_target(&env, &scope, HarnessId::Claude, "guard")
        else {
            panic!("claude hooks are script targets");
        };
        assert_eq!(path, PathBuf::from("/p/.claude/hooks/guard.sh"));
        assert_eq!(
            command,
            "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/guard.sh\""
        );
        assert_eq!(registry, PathBuf::from("/p/.claude/settings.json"));
        assert_eq!(feature, None);

        let Some(HookTarget::Script { command, .. }) =
            hook_target(&env, &Scope::Global, HarnessId::Claude, "guard")
        else {
            panic!("claude hooks are script targets");
        };
        assert_eq!(command, "bash /h/.claude/hooks/guard.sh");
    }

    #[test]
    fn codex_registers_in_hooks_json_and_enables_the_feature() {
        let env = Env::fake("/h", FakeOs::Linux);
        let scope = Scope::Project {
            root: PathBuf::from("/p"),
        };
        let Some(HookTarget::Script {
            command,
            registry,
            feature,
            ..
        }) = hook_target(&env, &scope, HarnessId::Codex, "guard")
        else {
            panic!("codex hooks are script targets");
        };
        assert_eq!(
            command,
            "bash \"$(git rev-parse --show-toplevel)/.codex/hooks/guard.sh\""
        );
        assert_eq!(registry, PathBuf::from("/p/.codex/hooks.json"));
        assert_eq!(feature, Some(PathBuf::from("/p/.codex/config.toml")));
    }

    #[test]
    fn instruction_references_are_scope_relative_and_cursor_is_project_only() {
        let env = Env::fake("/h", FakeOs::Linux);
        let scope = Scope::Project {
            root: PathBuf::from("/p"),
        };
        let Some(HookTarget::Instruction {
            path, reference, ..
        }) = hook_target(&env, &scope, HarnessId::Opencode, "guard")
        else {
            panic!("opencode hooks are instruction targets");
        };
        assert_eq!(
            path,
            PathBuf::from("/p/.opencode/instructions/vstack-hook-guard.md")
        );
        assert_eq!(reference, ".opencode/instructions/vstack-hook-guard.md");

        let Some(HookTarget::Instruction { reference, .. }) =
            hook_target(&env, &Scope::Global, HarnessId::Opencode, "guard")
        else {
            panic!("opencode hooks are instruction targets");
        };
        assert_eq!(reference, "instructions/vstack-hook-guard.md");

        assert_eq!(
            hook_target(&env, &scope, HarnessId::Cursor, "guard"),
            Some(HookTarget::Rule {
                path: PathBuf::from("/p/.cursor/rules/safety-guard.mdc"),
            })
        );
        assert_eq!(
            hook_target(&env, &Scope::Global, HarnessId::Cursor, "guard"),
            None
        );
        assert_eq!(hook_target(&env, &scope, HarnessId::Pi, "guard"), None);
    }
}
