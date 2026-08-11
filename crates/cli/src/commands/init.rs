use std::fs;
use std::path::Path;

use super::{CliResult, out, say};

/// Maintainer scaffolding: create a source-catalog item skeleton in the
/// current directory (v1 contract: no name → usage + exit 0; a name
/// without --kind, or with '/' or a leading '-', → error).
pub fn run(name: Option<String>, kind: Option<String>) -> CliResult {
    let Some(name) = name else {
        say("usage: vstack init <name> --kind agent|skill|hook");
        return Ok(());
    };
    let Some(kind) = kind else {
        return Err("pass --kind agent|skill|hook".into());
    };
    if name.contains('/') || name.starts_with('-') {
        return Err("item names must not contain '/' or start with '-'".into());
    }
    let cwd = std::env::current_dir()?;
    match kind.as_str() {
        "agent" | "agents" | "a" => {
            let path = cwd.join("agents").join(format!("{name}.md"));
            write_new(
                &path,
                &format!(
                    "---\nname: {name}\ndescription: What this agent is for. Trigger conditions.\nmodel: sonnet\nrole: engineer\n---\n\n# {name}\n\nOperating instructions.\n"
                ),
            )?;
            out(&format!("created {}", path.display()));
        }
        "skill" | "skills" | "s" => {
            let path = cwd.join("skills").join(&name).join("SKILL.md");
            write_new(
                &path,
                &format!(
                    "---\nname: {name}\ndescription: When to reach for this skill.\n---\n\n# {name}\n\nHow to use it.\n"
                ),
            )?;
            out(&format!("created {}", path.display()));
        }
        "hook" | "hooks" | "h" => {
            let path = cwd.join("hooks").join(format!("{name}.sh"));
            write_new(
                &path,
                &format!(
                    "#!/usr/bin/env bash\n# ---\n# name: {name}\n# event: PreToolUse\n# matcher: Bash\n# description: What this hook protects against.\n# ---\nset -euo pipefail\nexit 0\n"
                ),
            )?;
            out(&format!("created {}", path.display()));
        }
        other => return Err(format!("unknown --kind '{other}' (agent | skill | hook)").into()),
    }
    Ok(())
}

fn write_new(path: &Path, content: &str) -> CliResult {
    if path.exists() {
        return Err(format!("{} already exists", path.display()).into());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}
