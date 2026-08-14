use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::harness::{HarnessAdapter, Surface, all_adapters};
use crate::model::{DetectedHarness, FileState, ItemKind, ObservedItem, Scope};
use crate::settings::AppSettings;

mod copilot;
mod files;
mod hooks;
pub(crate) mod jsonc;
mod plugins;
mod provenance;
mod readers;

/// One parsed entry from a structured surface, before it becomes an
/// `ObservedItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    pub name: String,
    pub enabled: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub harnesses: Vec<DetectedHarness>,
    pub items: Vec<ObservedItem>,
    /// Registered projects whose directory is gone — flagged, never dropped.
    pub missing_projects: Vec<PathBuf>,
    /// Unreadable or unparsable surfaces; truth the scan could not reach.
    pub warnings: Vec<String>,
}

/// Read-only truth of this machine: every kind, every harness, global scope
/// plus every registered project.
pub fn scan(env: &Env, settings: &AppSettings) -> ScanResult {
    let mut scopes = vec![Scope::Global];
    scopes.extend(
        settings
            .projects
            .iter()
            .map(|p| Scope::Project { root: p.clone() }),
    );
    scan_scopes(env, &settings.harness_roots, &scopes)
}

/// The same engine over an explicit scope list — the CLI scans the current
/// project + global, the app scans everything registered.
pub fn scan_scopes(
    env: &Env,
    harness_roots: &std::collections::BTreeMap<String, PathBuf>,
    scopes: &[Scope],
) -> ScanResult {
    let mut result = ScanResult {
        harnesses: Vec::new(),
        items: Vec::new(),
        missing_projects: Vec::new(),
        warnings: Vec::new(),
    };
    let mut provenance = provenance::OriginCache::default();

    for scope in scopes {
        match scope {
            Scope::Global => {
                for adapter in all_adapters() {
                    let root = harness_roots
                        .get(adapter.id().name())
                        .cloned()
                        .unwrap_or_else(|| adapter.default_global_root(env));
                    if let Some(found) = adapter.detect(env, &root) {
                        result.harnesses.push(found);
                    }
                    for kind in ItemKind::ALL {
                        for surface in adapter.global_surfaces(kind, &root, env) {
                            scan_surface(
                                adapter,
                                kind,
                                Scope::Global,
                                &surface,
                                env,
                                &mut provenance,
                                &mut result,
                            );
                        }
                    }
                }
            }
            Scope::Project { root: project } => {
                if !project.is_dir() {
                    result.missing_projects.push(project.clone());
                    continue;
                }
                for adapter in all_adapters() {
                    for kind in ItemKind::ALL {
                        for surface in adapter.project_surfaces(kind, project, env) {
                            scan_surface(
                                adapter,
                                kind,
                                scope.clone(),
                                &surface,
                                env,
                                &mut provenance,
                                &mut result,
                            );
                        }
                    }
                }
            }
        }
    }

    result
}

fn scan_surface(
    adapter: &dyn HarnessAdapter,
    kind: ItemKind,
    scope: Scope,
    surface: &Surface,
    env: &Env,
    provenance: &mut provenance::OriginCache,
    result: &mut ScanResult,
) {
    match surface {
        Surface::FileDir { dir, exts, prefix } => {
            for found in files::scan_file_dir(dir, exts, *prefix) {
                result.items.push(ObservedItem {
                    kind,
                    name: found.name,
                    harness: adapter.id(),
                    scope: scope.clone(),
                    file_state: files::state_of(&found.path),
                    origin: provenance.origin_of(&found.path),
                    path: found.path,
                    enabled: Some(found.enabled),
                    description: found.description,
                    modified_at: found.modified_at,
                });
            }
        }
        Surface::SubdirPerItem { dir, marker } => {
            for found in files::scan_subdirs(dir, marker) {
                result.items.push(ObservedItem {
                    kind,
                    name: found.name,
                    harness: adapter.id(),
                    scope: scope.clone(),
                    file_state: files::state_of(&found.path),
                    origin: provenance.origin_of(&found.path),
                    path: found.path,
                    enabled: Some(found.enabled),
                    description: found.description,
                    modified_at: found.modified_at,
                });
            }
        }
        Surface::Structured { path, reader } => {
            if path.exists() {
                scan_structured_file(adapter, kind, &scope, path, reader, env, result);
            }
        }
        Surface::StructuredDir { dir, ext, reader } => {
            for path in files::scan_documents(dir, ext) {
                scan_structured_file(adapter, kind, &scope, &path, reader, env, result);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_structured_file(
    adapter: &dyn HarnessAdapter,
    kind: ItemKind,
    scope: &Scope,
    path: &std::path::Path,
    reader: &crate::harness::Reader,
    env: &Env,
    result: &mut ScanResult,
) {
    match readers::read_structured(path, reader, env) {
        Ok(entries) => {
            for entry in entries {
                result.items.push(ObservedItem {
                    kind,
                    name: entry.name,
                    harness: adapter.id(),
                    scope: scope.clone(),
                    path: path.to_path_buf(),
                    file_state: FileState::ConfigEntry,
                    enabled: entry.enabled,
                    origin: None,
                    description: entry.description,
                    // One file holds every entry of this kind; its mtime
                    // would describe all of them at once, not this one.
                    modified_at: None,
                });
            }
        }
        Err(message) => result
            .warnings
            .push(format!("{}: {message}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;
    use crate::model::HarnessId;
    use std::fs;

    /// One fixture home exercising every adapter: claude agent + skill +
    /// hooks + mcp, codex agent + prompt, opencode mcp with a disabled
    /// entry, pi package, and a registered project with a shared skill tree.
    #[test]
    fn scans_a_realistic_machine() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let env = Env::fake(home, FakeOs::Linux);

        fs::create_dir_all(home.join(".claude/agents")).unwrap();
        fs::write(
            home.join(".claude/agents/orch.md"),
            "---\ndescription: boss\n---\n",
        )
        .unwrap();
        fs::create_dir_all(home.join(".claude/skills/github")).unwrap();
        fs::write(home.join(".claude/skills/github/SKILL.md"), "# gh").unwrap();
        fs::write(
            home.join(".claude/settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"bash guard.sh"}]}]}}"#,
        )
        .unwrap();
        fs::write(
            home.join(".claude.json"),
            r#"{"mcpServers":{"github":{"command":"gh-mcp"}}}"#,
        )
        .unwrap();

        fs::create_dir_all(home.join(".codex/agents")).unwrap();
        fs::write(
            home.join(".codex/agents/rust.toml"),
            "description = \"rust dev\"\n",
        )
        .unwrap();
        fs::create_dir_all(home.join(".codex/prompts")).unwrap();
        fs::write(home.join(".codex/prompts/ship.md"), "ship it").unwrap();

        fs::create_dir_all(home.join(".config/opencode")).unwrap();
        fs::write(
            home.join(".config/opencode/opencode.json"),
            r#"{"mcp":{"db":{"type":"local","enabled":false,"command":["db"]}}}"#,
        )
        .unwrap();

        fs::create_dir_all(home.join(".pi/agent")).unwrap();
        fs::write(
            home.join(".pi/agent/settings.json"),
            r#"{"packages":["npm:@vanillagreen/pi-hooks@1.2.0","./packages/pi-tmux"]}"#,
        )
        .unwrap();

        let project = home.join("dev/app");
        fs::create_dir_all(project.join(".agents/skills/deploy")).unwrap();
        fs::write(project.join(".agents/skills/deploy/SKILL.md"), "# d").unwrap();

        let mut settings = AppSettings::default();
        settings.projects.push(project.clone());
        settings.projects.push(home.join("dev/vanished"));

        let result = scan(&env, &settings);

        assert_eq!(result.warnings, Vec::<String>::new());
        assert_eq!(result.missing_projects, [home.join("dev/vanished")]);

        let detected: Vec<_> = result.harnesses.iter().map(|h| h.harness).collect();
        assert_eq!(
            detected,
            [
                HarnessId::Claude,
                HarnessId::Codex,
                HarnessId::Opencode,
                HarnessId::Pi
            ]
        );

        let find = |kind: ItemKind, name: &str| {
            result
                .items
                .iter()
                .filter(|i| i.kind == kind && i.name == name)
                .collect::<Vec<_>>()
        };

        let agent = find(ItemKind::Agent, "orch");
        assert_eq!(agent.len(), 1);
        assert_eq!(agent[0].description.as_deref(), Some("boss"));

        assert_eq!(find(ItemKind::Skill, "github").len(), 1);
        assert_eq!(find(ItemKind::Hook, "PreToolUse:Bash:guard").len(), 1);
        assert_eq!(find(ItemKind::McpServer, "github").len(), 1);
        assert_eq!(
            find(ItemKind::Agent, "rust")[0].description.as_deref(),
            Some("rust dev")
        );
        assert_eq!(find(ItemKind::Command, "ship").len(), 1);

        let db = find(ItemKind::McpServer, "db");
        assert_eq!(db[0].enabled, Some(false));

        assert_eq!(
            find(ItemKind::PiExtension, "@vanillagreen/pi-hooks").len(),
            1
        );
        assert_eq!(find(ItemKind::PiExtension, "pi-tmux").len(), 1);

        // The shared .agents/skills tree surfaces once per harness, same path.
        let deploy = find(ItemKind::Skill, "deploy");
        assert_eq!(deploy.len(), 2);
        assert_eq!(deploy[0].path, deploy[1].path);
        let harnesses: Vec<_> = deploy.iter().map(|i| i.harness).collect();
        assert!(harnesses.contains(&HarnessId::Codex) && harnesses.contains(&HarnessId::Pi));
    }

    /// Gemini and Copilot installations are read the same way as everyone
    /// else's — and Copilot's reach into `.claude/` never becomes a second
    /// installation of a file that already belongs to Claude Code.
    #[test]
    fn sees_gemini_and_copilot_without_double_counting_claude_files() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let env = Env::fake(home, FakeOs::Linux);

        fs::create_dir_all(home.join(".gemini/agents")).unwrap();
        fs::write(
            home.join(".gemini/agents/plan.md"),
            "---\ndescription: planner\n---\n",
        )
        .unwrap();
        fs::write(
            home.join(".gemini/settings.json"),
            r#"{"mcpServers":{"docs":{"httpUrl":"https://docs.example"}},
                "hooks":{"BeforeTool":[{"matcher":"run_shell_command",
                "hooks":[{"type":"command","command":"bash audit.sh"}]}]}}"#,
        )
        .unwrap();
        fs::create_dir_all(home.join(".gemini/extensions/security")).unwrap();
        fs::write(
            home.join(".gemini/extensions/security/gemini-extension.json"),
            r#"{"name":"security"}"#,
        )
        .unwrap();

        fs::create_dir_all(home.join(".copilot/agents")).unwrap();
        fs::write(home.join(".copilot/agents/review.agent.md"), "---\n---\n").unwrap();

        let project = home.join("dev/app");
        fs::create_dir_all(project.join(".github/skills/deploy")).unwrap();
        fs::write(project.join(".github/skills/deploy/SKILL.md"), "# d").unwrap();
        fs::create_dir_all(project.join(".claude/skills/private")).unwrap();
        fs::write(project.join(".claude/skills/private/SKILL.md"), "# p").unwrap();

        let mut settings = AppSettings::default();
        settings.projects.push(project.clone());
        let result = scan(&env, &settings);

        assert_eq!(result.warnings, Vec::<String>::new());
        let detected: Vec<_> = result.harnesses.iter().map(|h| h.harness).collect();
        assert!(detected.contains(&HarnessId::Gemini) && detected.contains(&HarnessId::Copilot));

        let of = |harness: HarnessId| {
            result
                .items
                .iter()
                .filter(|i| i.harness == harness)
                .map(|i| (i.kind, i.name.as_str()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            of(HarnessId::Gemini),
            [
                (ItemKind::Agent, "plan"),
                (ItemKind::Hook, "BeforeTool:run_shell_command:audit"),
                (ItemKind::McpServer, "docs"),
                (ItemKind::Plugin, "security"),
            ]
        );
        // The `.agent.md` pair is one extension, not part of the name; the
        // skill under `.claude/` stays Claude Code's alone.
        assert_eq!(
            of(HarnessId::Copilot),
            [(ItemKind::Agent, "review"), (ItemKind::Skill, "deploy")]
        );
    }
}
