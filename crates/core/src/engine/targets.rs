use std::path::PathBuf;

use crate::env::Env;
use crate::harness::{Enforcement, adapter, capabilities};
use crate::model::{HarnessId, ItemKind, Scope};

/// What installing a hook on this harness actually buys. A tool that only
/// reads the file must never be presented as one that acts on it: the
/// warning travels with the plan, the preview, and the audit page.
pub(super) fn advisory_notice(harness: HarnessId, name: &str) -> Option<super::ItemWarning> {
    let tool = harness.display_name();
    (capabilities(harness, ItemKind::Hook).enforcement == Enforcement::Advisory).then(|| {
        super::ItemWarning {
            kind: ItemKind::Hook,
            name: name.to_owned(),
            harness: Some(harness),
            message: format!(
                "this protection is advisory on {tool} — it installs as text the model may ignore, not a check the tool runs"
            ),
            remediation: Some(format!(
                "keep it for the tools that run hooks — Claude Code, Codex, Gemini CLI, GitHub Copilot — or accept it as guidance on {tool}"
            )),
        }
    })
}

/// Which shape a registry file speaks. Claude, codex, cursor and Gemini all
/// take the same matcher-with-handlers object; Copilot's hook files are a
/// `{version, hooks}` document whose entries carry the command themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HookFormat {
    Nested,
    Copilot,
}

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
        format: HookFormat,
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
                format: HookFormat::Nested,
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
                format: HookFormat::Nested,
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
        // Gemini registers in the `hooks` key of its settings.json, in the
        // same matcher-plus-handlers shape claude's takes (matrix §1). The
        // script is ours to place; `.gemini/hooks` is not a surface Gemini
        // scans, so nothing reads it except the command we register.
        HarnessId::Gemini => {
            let root = match scope {
                Scope::Global => adapter(harness).default_global_root(env),
                Scope::Project { root } => root.join(".gemini"),
            };
            let path = root.join("hooks").join(format!("{name}.sh"));
            let command = match scope {
                Scope::Global => format!("bash {}", path.display()),
                // Gemini documents no project-directory variable, so the
                // path resolves through the repo root itself.
                Scope::Project { .. } => {
                    format!("bash \"$(git rev-parse --show-toplevel)/.gemini/hooks/{name}.sh\"")
                }
            };
            Some(HookTarget::Script {
                path,
                command,
                registry: root.join("settings.json"),
                format: HookFormat::Nested,
                feature: None,
            })
        }
        // pi hooks belong to the pi-hooks extension, not to files we manage.
        HarnessId::Pi => None,
        HarnessId::Copilot => Some(copilot_hook(env, scope, name)),
    }
}

/// Copilot loads every `*.json` under its hooks directory as a hook document
/// of its own, so each hook gets a file rather than a shared one — and the
/// script beside it is invisible to that glob (matrix §2, §R5). Only a file
/// is a switch: an entry inline in a settings file has no flag to flip.
fn copilot_hook(env: &Env, scope: &Scope, name: &str) -> HookTarget {
    let (dir, command) = match scope {
        Scope::Global => {
            let dir = adapter(HarnessId::Copilot)
                .default_global_root(env)
                .join("hooks");
            let command = format!("bash {}", dir.join(format!("{name}.sh")).display());
            (dir, command)
        }
        Scope::Project { root } => (
            root.join(".github/hooks"),
            format!("bash \"$(git rev-parse --show-toplevel)/.github/hooks/{name}.sh\""),
        ),
    };
    HookTarget::Script {
        path: dir.join(format!("{name}.sh")),
        command,
        registry: dir.join(format!("{name}.json")),
        format: HookFormat::Copilot,
        feature: None,
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

/// The file `mcpServers` entries are written to. Claude's project servers
/// belong to the repo's `.mcp.json` and its global ones to the user file;
/// Gemini keeps both in the settings file for that scope (matrix §1).
pub(super) fn mcp_registry(env: &Env, scope: &Scope, harness: HarnessId) -> Option<PathBuf> {
    match harness {
        HarnessId::Claude => Some(match scope {
            Scope::Global => env.home.join(".claude.json"),
            Scope::Project { root } => root.join(".mcp.json"),
        }),
        HarnessId::Gemini => Some(crate::harness::gemini::settings::settings_file(env, scope)),
        // Copilot reads a repository's servers from `.github/mcp.json` and a
        // machine's from its own config root (matrix §2). A `.mcp.json` at
        // the repo root is Claude Code's file, which Copilot also reads —
        // writing there would count one declaration as two installations.
        HarnessId::Copilot => Some(match scope {
            Scope::Global => adapter(harness)
                .default_global_root(env)
                .join("mcp-config.json"),
            Scope::Project { root } => root.join(".github/mcp.json"),
        }),
        _ => None,
    }
}

/// The settings file whose `enabledPlugins` map a plugin toggle writes.
/// Every harness that reads such a map has one of its own — a declaration
/// aimed at one tool must never land in another tool's settings.
pub(super) fn plugin_settings(env: &Env, scope: &Scope, harness: HarnessId) -> Option<PathBuf> {
    match harness {
        HarnessId::Claude => Some(claude_settings(env, scope)),
        HarnessId::Copilot => Some(crate::harness::copilot::settings::settings_file(env, scope)),
        _ => None,
    }
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
            ..
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

    /// Copilot's hook file and the script it runs sit side by side, and the
    /// file is the one its loader globs for.
    #[test]
    fn a_copilot_hook_gets_a_document_of_its_own_beside_its_script() {
        let env = Env::fake("/h", FakeOs::Linux);
        let scope = Scope::Project {
            root: PathBuf::from("/p"),
        };
        let Some(HookTarget::Script {
            path,
            command,
            registry,
            format,
            feature,
        }) = hook_target(&env, &scope, HarnessId::Copilot, "audit")
        else {
            panic!("copilot hooks are script targets");
        };
        assert_eq!(path, PathBuf::from("/p/.github/hooks/audit.sh"));
        assert_eq!(
            command,
            "bash \"$(git rev-parse --show-toplevel)/.github/hooks/audit.sh\""
        );
        assert_eq!(registry, PathBuf::from("/p/.github/hooks/audit.json"));
        assert_eq!(format, HookFormat::Copilot);
        assert_eq!(feature, None);

        let Some(HookTarget::Script {
            command, registry, ..
        }) = hook_target(&env, &Scope::Global, HarnessId::Copilot, "audit")
        else {
            panic!("copilot hooks are script targets");
        };
        assert_eq!(command, "bash /h/.copilot/hooks/audit.sh");
        assert_eq!(registry, PathBuf::from("/h/.copilot/hooks/audit.json"));
    }
}
