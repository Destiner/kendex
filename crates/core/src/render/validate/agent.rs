use super::{Finding, frontmatter_map};
use crate::frontmatter::Value;

const SANDBOX_MODES: [&str; 3] = ["read-only", "workspace-write", "danger-full-access"];
const MODES: [&str; 3] = ["primary", "subagent", "all"];
const PERMISSION_VALUES: [&str; 3] = ["allow", "ask", "deny"];
/// Every key Cursor's rule loader reads. The rest are folklore.
const CURSOR_KEYS: [&str; 3] = ["description", "globs", "alwaysApply"];

/// Codex agents are TOML. A file that does not parse is skipped in silence,
/// and a missing required key is an agent Codex never offers.
pub(super) fn codex(text: &str) -> Vec<Finding> {
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(problem) => {
            return vec![Finding::breakage(
                format!("Codex reads agents as TOML and this one does not parse — {problem}"),
                "check the agent's frontmatter and body in the catalog for stray quotes or control characters",
            )];
        }
    };
    let mut findings = Vec::new();
    for key in ["name", "description", "developer_instructions"] {
        let filled = table
            .get(key)
            .and_then(toml::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
        if filled {
            continue;
        }
        findings.push(Finding::breakage(
            format!("the Codex agent has no `{key}`, so Codex will not load it"),
            match key {
                "developer_instructions" => {
                    "write the agent a body in the catalog — there is nothing to instruct it with"
                        .to_owned()
                }
                _ => format!("add `{key}:` to the agent's frontmatter in the catalog"),
            },
        ));
    }
    if let Some(mode) = table.get("sandbox_mode") {
        let shown = mode.as_str().unwrap_or("not text");
        if !SANDBOX_MODES.contains(&shown) {
            findings.push(Finding::breakage(
                format!("`sandbox_mode = \"{shown}\"` is not a sandbox Codex knows"),
                format!("use one of {}", SANDBOX_MODES.join(", ")),
            ));
        }
    }
    findings
}

/// OpenCode reads agent frontmatter strictly: a mode or permission value it
/// does not know drops the agent rather than defaulting it.
pub(super) fn opencode(text: &str) -> Vec<Finding> {
    let map = match frontmatter_map(text, "OpenCode") {
        Ok(map) => map,
        Err(finding) => return vec![finding],
    };
    let mut findings = Vec::new();
    if let Some(mode) = map.get("mode").and_then(Value::as_str)
        && !MODES.contains(&mode)
    {
        findings.push(Finding::breakage(
            format!("`mode: {mode}` is not a mode OpenCode knows"),
            format!("set the agent's mode to one of {}", MODES.join(", ")),
        ));
    }
    if let Some(model) = map.get("model").and_then(Value::as_str)
        && !model.contains('/')
    {
        findings.push(Finding::advisory(
            format!("`model: {model}` names no provider, so OpenCode falls back to its default"),
            "write the model as `provider/model`, or leave it out to inherit OpenCode's default",
        ));
    }
    match map.get("permission") {
        None => {}
        Some(Value::Map(permissions)) => {
            for (key, value) in permissions.entries() {
                let shown = value.as_str().unwrap_or("a nested block");
                if !PERMISSION_VALUES.contains(&shown) {
                    findings.push(Finding::breakage(
                        format!(
                            "permission `{key}` is set to `{shown}`, which OpenCode cannot read"
                        ),
                        format!("set it to one of {}", PERMISSION_VALUES.join(", ")),
                    ));
                }
            }
        }
        Some(_) => findings.push(Finding::breakage(
            "`permission:` is not a block of permission names",
            "write permission as indented `<name>: allow|ask|deny` lines",
        )),
    }
    findings
}

/// Claude registers an agent under its frontmatter name, so a name that
/// disagrees with the declared one answers to something nobody typed.
pub(super) fn claude(name: &str, text: &str) -> Vec<Finding> {
    let map = match frontmatter_map(text, "Claude Code") {
        Ok(map) => map,
        Err(finding) => return vec![finding],
    };
    let declared = map
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if declared.is_empty() {
        return vec![Finding::breakage(
            format!("the Claude agent for `{name}` has no name, so nothing can call it"),
            format!("add `name: {name}` to the agent's frontmatter in the catalog"),
        )];
    }
    if declared != name {
        return vec![Finding::breakage(
            format!(
                "the agent installs as `{name}` but calls itself `{declared}`, so Claude answers to the wrong one"
            ),
            format!("rename it to `{name}` in the catalog, or declare the agent as `{declared}`"),
        )];
    }
    Vec::new()
}

pub(super) fn cursor(text: &str) -> Vec<Finding> {
    let map = match frontmatter_map(text, "Cursor") {
        Ok(map) => map,
        Err(finding) => return vec![finding],
    };
    map.entries()
        .filter(|(key, _)| !CURSOR_KEYS.contains(key))
        .map(|(key, _)| {
            Finding::advisory(
                format!("Cursor ignores `{key}:` in a rule file"),
                format!(
                    "keep rule frontmatter to {} — every other key is folklore",
                    CURSOR_KEYS.join(", ")
                ),
            )
        })
        .collect()
}
