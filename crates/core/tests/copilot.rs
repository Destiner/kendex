//! GitHub Copilot as a managed tool: an agent, a skill, a hook and an MCP
//! server each install in the shape Copilot reads, switch on and off without
//! losing anything, and come off disk on request — leaving the keys around
//! ours in its files untouched.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use vstack_core::apply;
use vstack_core::engine::{EngineReport, audit, ops};
use vstack_core::env::{Env, FakeOs};
use vstack_core::model::Scope;

const AGENT: &str = "---\nname: rust\ndescription: Rust engineer\nmodel: opus\nrole: engineer\n---\nUse the Grep tool.\n";

const AUDIT_HOOK: &str = "#!/usr/bin/env bash\n# ---\n# name: audit\n# event: PreToolUse\n# matcher: Bash\n# description: log shell commands\n# timeout: 10\n# ---\nexit 0\n";

const GH_MCP: &str = "command = \"gh-mcp\"\nargs = [\"--stdio\"]\n";

const COMMAND: &str = "---\ndescription: Ship the branch\n---\n\nRun the checklist.\n";

struct Fixture {
    _tmp: tempfile::TempDir,
    env: Env,
    scope: Scope,
    project: PathBuf,
}

/// A copilot-only project whose catalog carries one of everything;
/// `declarations` is appended to the manifest verbatim.
#[allow(clippy::unwrap_used)]
fn fixture(declarations: &str) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let env = Env::fake(&home, FakeOs::Linux);
    let project = home.join("dev/app");
    fs::create_dir_all(project.join(".github")).unwrap();
    fs::create_dir_all(home.join(".copilot")).unwrap();

    let source = home.join("catalog");
    for dir in ["agents", "hooks", "mcp", "commands", "skills/deploy"] {
        fs::create_dir_all(source.join(dir)).unwrap();
    }
    fs::write(
        source.join("skills/deploy/SKILL.md"),
        "---\nname: deploy\ndescription: Ship it\n---\n\nSteps.\n",
    )
    .unwrap();
    fs::write(source.join("agents/rust.md"), AGENT).unwrap();
    fs::write(source.join("hooks/audit.sh"), AUDIT_HOOK).unwrap();
    fs::write(source.join("mcp/gh.toml"), GH_MCP).unwrap();
    fs::write(source.join("commands/ship.md"), COMMAND).unwrap();

    fs::write(
        project.join("vstack.toml"),
        format!(
            "schema = 3\n\n[sources.cat]\npath = \"{}\"\n\n[install]\nharnesses = [\"copilot\"]\nmethod = \"symlink\"\n\n{declarations}",
            source.display()
        ),
    )
    .unwrap();

    Fixture {
        env,
        scope: Scope::Project {
            root: project.clone(),
        },
        project,
        _tmp: tmp,
    }
}

#[allow(clippy::unwrap_used)]
fn apply_now(f: &Fixture) -> EngineReport {
    let report = audit(&f.env, &f.scope).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();
    report
}

#[allow(clippy::unwrap_used)]
fn toggle(f: &Fixture, name: &str, enabled: bool) {
    let report = ops::toggle(&f.env, &f.scope, &[name.to_owned()], enabled).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();
}

#[allow(clippy::unwrap_used)]
fn remove(f: &Fixture, name: &str) {
    let report = ops::remove(&f.env, &f.scope, &[name.to_owned()], false).unwrap();
    apply::execute(&f.env, &report.plan, None).unwrap();
}

#[allow(clippy::unwrap_used)]
fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[allow(clippy::unwrap_used)]
fn is_clean(f: &Fixture) -> bool {
    audit(&f.env, &f.scope).unwrap().drift.is_empty()
}

#[test]
#[allow(clippy::unwrap_used)]
fn an_agent_installs_with_copilots_double_extension_and_toggles_by_rename() {
    let f = fixture("[agents.rust]\nsource = \"cat\"\n");
    apply_now(&f);

    let file = f.project.join(".github/agents/rust.agent.md");
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.starts_with("---\nname: rust\ndescription: \"Rust engineer\"\n"));
    // Which models a user can reach depends on their plan and their
    // organization, so vstack pins none of them.
    assert!(text.contains("model: auto\n"), "{text}");
    assert!(text.contains("Use the grep tool."), "{text}");
    assert!(is_clean(&f));

    toggle(&f, "rust", false);
    assert!(!file.exists());
    let parked = f.project.join(".github/agents/rust.agent.md.disabled");
    assert_eq!(fs::read_to_string(&parked).unwrap(), text);
    assert!(is_clean(&f));

    toggle(&f, "rust", true);
    assert!(file.is_file() && !parked.exists());

    remove(&f, "rust");
    assert!(!file.exists() && !parked.exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_skill_installs_into_copilots_own_skills_directory() {
    let f = fixture("[skills.deploy]\nsource = \"cat\"\n");
    apply_now(&f);

    let marker = f.project.join(".github/skills/deploy/SKILL.md");
    assert!(fs::read_to_string(&marker).unwrap().contains("Steps."));
    assert!(is_clean(&f));

    toggle(&f, "deploy", false);
    assert!(!marker.exists());
    assert!(
        f.project
            .join(".github/skills/deploy/SKILL.md.disabled")
            .exists()
    );
    assert!(is_clean(&f));

    remove(&f, "deploy");
    assert!(!f.project.join(".github/skills/deploy").exists());
}

/// Copilot globs `*.json` out of its hooks directory, so each hook gets a
/// document of its own and the script beside it is invisible to that glob.
#[test]
#[allow(clippy::unwrap_used)]
fn a_hook_registers_in_a_hook_file_of_its_own() {
    let f = fixture("[hooks.audit]\nsource = \"cat\"\n");
    apply_now(&f);

    let script = f.project.join(".github/hooks/audit.sh");
    let registry = f.project.join(".github/hooks/audit.json");
    assert!(script.is_file());
    let registered = json(&registry);
    assert_eq!(registered["version"], 1);
    let entry = &registered["hooks"]["preToolUse"][0];
    assert_eq!(entry["type"], "command");
    assert_eq!(
        entry["bash"],
        "bash \"$(git rev-parse --show-toplevel)/.github/hooks/audit.sh\""
    );
    // The same `Bash` the source declares, said in Copilot's tool names —
    // a case-sensitive regex of Claude's spelling would match nothing.
    assert_eq!(entry["matcher"], "bash");
    // Copilot reads this one in seconds, which is what the source declares.
    assert_eq!(entry["timeoutSec"], 10);
    assert!(is_clean(&f));

    toggle(&f, "audit", false);
    assert!(json(&registry).get("hooks").is_none());
    assert!(f.project.join(".github/hooks/audit.sh.disabled").is_file());
    assert!(is_clean(&f));

    toggle(&f, "audit", true);
    assert_eq!(json(&registry), registered);

    remove(&f, "audit");
    assert!(!script.exists());
    assert!(json(&registry).get("hooks").is_none());
}

/// What vstack writes is what vstack reads back: the hook file lands in the
/// directory Copilot globs, and the scan finds the entry inside it rather
/// than the file it happens to be called.
#[test]
#[allow(clippy::unwrap_used)]
fn a_registered_hook_is_read_back_from_copilots_own_directory() {
    let f = fixture("[hooks.audit]\nsource = \"cat\"\n");
    apply_now(&f);

    let scanned = vstack_core::scan::scan_scopes(
        &f.env,
        &std::collections::BTreeMap::new(),
        std::slice::from_ref(&f.scope),
    );
    assert_eq!(scanned.warnings, Vec::<String>::new());
    let hooks: Vec<_> = scanned
        .items
        .iter()
        .filter(|item| item.harness == vstack_core::model::HarnessId::Copilot)
        .map(|item| (item.name.as_str(), item.enabled))
        .collect();
    assert_eq!(hooks, [("preToolUse:bash:audit", Some(true))]);
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_project_server_is_declared_in_githubs_mcp_file_and_left_alone_otherwise() {
    let f = fixture("[mcp-servers.gh]\nsource = \"cat\"\n\n[mcp-servers.docs]\nsource = \"cat\"\n");
    fs::write(
        f._tmp.path().join("catalog/mcp/docs.toml"),
        "transport = \"http\"\nurl = \"https://docs.example\"\n",
    )
    .unwrap();
    let mcp = f.project.join(".github/mcp.json");
    fs::write(
        &mcp,
        "{\n  \"mcpServers\": {\n    \"other\": {\"command\": \"x\"}\n  }\n}\n",
    )
    .unwrap();
    apply_now(&f);

    let installed = json(&mcp);
    assert_eq!(installed["mcpServers"]["other"]["command"], "x");
    // Copilot names the transport on the entry itself: a command server is
    // `local`, a url server says which protocol it speaks.
    assert_eq!(installed["mcpServers"]["gh"]["type"], "local");
    assert_eq!(installed["mcpServers"]["gh"]["command"], "gh-mcp");
    assert_eq!(installed["mcpServers"]["gh"]["args"][0], "--stdio");
    assert_eq!(installed["mcpServers"]["docs"]["type"], "http");
    assert_eq!(
        installed["mcpServers"]["docs"]["url"],
        "https://docs.example"
    );
    assert!(is_clean(&f));

    // Switching one off is the direction a repository can always express;
    // the declaration is what brings it back, so nothing is lost.
    toggle(&f, "gh", false);
    let off = json(&mcp);
    assert!(off["mcpServers"].get("gh").is_none());
    assert_eq!(off["mcpServers"]["other"]["command"], "x");
    assert!(is_clean(&f));

    toggle(&f, "gh", true);
    assert_eq!(json(&mcp)["mcpServers"]["gh"]["command"], "gh-mcp");

    remove(&f, "gh");
    let after = json(&mcp);
    assert!(after["mcpServers"].get("gh").is_none());
    assert_eq!(after["mcpServers"]["other"]["command"], "x");
    assert!(is_clean(&f));
}

/// No Copilot product reads a file-backed slash command, so a command
/// declared for it produces nothing at all rather than a dead file.
#[test]
#[allow(clippy::unwrap_used)]
fn a_command_declared_for_copilot_writes_nothing() {
    let f = fixture("[commands.ship]\nsource = \"cat\"\n");
    apply_now(&f);
    assert!(!f.project.join(".github/commands").exists());
    assert!(!f.project.join(".github/prompts").exists());
    assert!(is_clean(&f));
}
