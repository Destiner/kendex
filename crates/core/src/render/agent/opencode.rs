use super::{EffectiveAgent, GENERATED_BANNER, hooks_prose, model_id_for, skills_prose};
use crate::manifest::FrontmatterOverrides;
use crate::model::Scope;

/// OpenCode agent: YAML frontmatter + markdown system prompt. Tools are
/// controlled by a deny-only `permission:` map keyed by permission name, not
/// tool name, so every deny-tool is translated first.
pub fn generate(agent: &EffectiveAgent) -> String {
    let source = agent.source;
    let o = &agent.overrides;
    let mut out = String::from("---\n");
    out.push_str(&format!("description: {}\n", yaml_str(&source.description)));
    let mode = mode(o);
    out.push_str(&format!("mode: {mode}\n"));
    let model = o.model.as_deref().unwrap_or(&source.model);
    out.push_str(&format!("model: {}\n", model_id_for("openai", model)));
    if let Some(color) = o
        .color
        .as_deref()
        .or(source.color.as_deref())
        .and_then(color_hex)
    {
        out.push_str(&format!("color: {}\n", yaml_str(&color)));
    }
    let effort = o
        .model_reasoning_effort
        .as_deref()
        .or(o.effort.as_deref())
        .or(source.effort.as_deref())
        .filter(|effort| !is_none_value(effort));
    if let Some(effort) = effort {
        out.push_str(&format!(
            "options:\n  reasoningEffort: {effort}\n  reasoningSummary: auto\n  textVerbosity: medium\n"
        ));
    }
    let denied = denied_permissions(agent, mode);
    if !denied.is_empty() {
        out.push_str("permission:\n");
        for permission in denied {
            out.push_str(&format!("  {permission}: deny\n"));
        }
    }
    out.push_str("---\n\n");
    out.push_str(&body(agent));
    out
}

/// `all` means "usable either way", which opencode spells `subagent`.
fn mode(o: &FrontmatterOverrides) -> &str {
    match o.mode.as_deref().map(str::trim) {
        Some(mode) if !mode.is_empty() && !mode.eq_ignore_ascii_case("all") => mode,
        _ => "subagent",
    }
}

/// Subagents never spawn further agents, and only the planner may interrupt
/// the user. Primary agents keep both.
fn denied_permissions(agent: &EffectiveAgent, mode: &str) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    if mode == "subagent" {
        tools.push("task".to_owned());
        if agent.source.name != "planner" {
            tools.push("question".to_owned());
        }
    }
    if let Some(extra) = &agent.overrides.deny_tools {
        tools.extend(extra.iter().cloned());
    }
    let mut permissions: Vec<String> = Vec::new();
    for permission in tools.iter().filter_map(|tool| permission_name(tool)) {
        if !permissions.contains(&permission) {
            permissions.push(permission);
        }
    }
    permissions
}

fn permission_name(tool: &str) -> Option<String> {
    let normalized = tool.trim().to_lowercase().replace(['_', '-'], "");
    let permission = match normalized.as_str() {
        "read" => "read",
        "edit" | "write" | "patch" | "applypatch" | "multiedit" | "notebookedit" => "edit",
        "glob" | "find" | "ls" | "list" => "glob",
        "grep" => "grep",
        "bash" | "shell" => "bash",
        "task" | "agent" | "subagent" | "spawnagent" | "spawnagentsoncsv" => "task",
        "skill" => "skill",
        "lsp" => "lsp",
        "question" => "question",
        "webfetch" | "websearch" | "web" | "webresearch" | "webanswer" | "codesearch" => "webfetch",
        "" => return None,
        _ => return Some(tool.trim().to_owned()),
    };
    Some(permission.to_owned())
}

fn color_hex(color: &str) -> Option<String> {
    let color = color.trim();
    if color.starts_with('#')
        && color.len() == 7
        && color.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
    {
        return Some(color.to_owned());
    }
    let hex = match color.to_lowercase().as_str() {
        "red" | "error" => "#ef4444",
        "green" | "success" => "#22c55e",
        "yellow" | "warning" => "#eab308",
        "orange" => "#f97316",
        "blue" | "primary" | "info" => "#3b82f6",
        "cyan" | "teal" => "#06b6d4",
        "purple" | "violet" | "magenta" | "accent" => "#a855f7",
        "pink" => "#ec4899",
        "secondary" => "#64748b",
        _ => return None,
    };
    Some(hex.to_owned())
}

fn body(agent: &EffectiveAgent) -> String {
    let mut out = format!("{GENERATED_BANNER}\n\n");
    if let Some(launch) = &agent.launch_instructions {
        out.push_str(&format!("## Launch Instructions\n\n{launch}\n\n"));
    }
    out.push_str(agent.source.body.trim_end());
    out.push('\n');
    let skill_root = match agent.scope {
        Scope::Global => "~/.config/opencode/skills",
        Scope::Project { .. } => ".opencode/skills",
    };
    if let Some(skills) = skills_prose(agent, skill_root) {
        out.push_str(&format!("\n{skills}"));
    }
    if let Some(hooks) = hooks_prose(agent) {
        out.push_str(&format!("\n{hooks}\n"));
    }
    if let Some(additional) = &agent.additional_instructions {
        out.push_str(&format!("\n## Additional Instructions\n\n{additional}\n"));
    }
    out
}

fn yaml_str(text: &str) -> String {
    if text.contains([':', '#', '"', '\'', '\n']) {
        return format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
    }
    text.to_owned()
}

fn is_none_value(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "" | "none" | "false" | "off" | "no"
    )
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAgent, parse_source_agent};
    use super::*;
    use crate::model::HarnessId;

    fn source(name: &str) -> SourceAgent {
        parse_source_agent(&format!(
            "---\nname: {name}\ndescription: OpenCode agent\nmodel: opus\nrole: engineer\ncolor: green\neffort: high\n---\nBody text.\n"
        ))
        .unwrap()
    }

    fn effective<'a>(source: &'a SourceAgent, scope: &'a Scope) -> EffectiveAgent<'a> {
        EffectiveAgent {
            source,
            harness: HarnessId::Opencode,
            scope,
            skills: vec![],
            overrides: FrontmatterOverrides::default(),
            launch_instructions: None,
            additional_instructions: None,
            custom_hooks: vec![],
        }
    }

    #[test]
    fn subagents_deny_task_and_questions_with_named_color_mapped_to_hex() {
        let source = source("reviewer");
        let scope = Scope::Global;
        let text = generate(&effective(&source, &scope));
        assert!(text.contains("mode: subagent\n"));
        assert!(text.contains("model: openai/gpt-5.6-sol\n"));
        assert!(text.contains("color: \"#22c55e\"\n"));
        assert!(text.contains("options:\n  reasoningEffort: high\n"));
        assert!(text.contains("permission:\n  task: deny\n  question: deny\n"));
    }

    #[test]
    fn planner_keeps_questions_and_hex_color_passes_through() {
        let source = source("planner");
        let scope = Scope::Global;
        let mut agent = effective(&source, &scope);
        agent.overrides = FrontmatterOverrides {
            color: Some("#336699".into()),
            ..FrontmatterOverrides::default()
        };
        let text = generate(&agent);
        assert!(text.contains("color: \"#336699\"\n"));
        assert!(text.contains("  task: deny\n"));
        assert!(!text.contains("question: deny"));
    }

    #[test]
    fn deny_tools_collapse_onto_permission_names() {
        let source = source("rust");
        let scope = Scope::Global;
        let mut agent = effective(&source, &scope);
        agent.overrides = FrontmatterOverrides {
            deny_tools: Some(vec![
                "write".into(),
                "apply_patch".into(),
                "subagent".into(),
                "WebSearch".into(),
                "mcp__custom".into(),
            ]),
            ..FrontmatterOverrides::default()
        };
        let text = generate(&agent);
        assert_eq!(text.matches("  edit: deny\n").count(), 1);
        assert_eq!(text.matches("  task: deny\n").count(), 1);
        assert!(text.contains("  webfetch: deny\n"));
        assert!(text.contains("  mcp__custom: deny\n"));
    }

    #[test]
    fn skills_and_hooks_render_as_prose_under_the_scope_root() {
        let source = source("rust");
        let scope = Scope::Project {
            root: "/tmp/proj".into(),
        };
        let mut agent = effective(&source, &scope);
        agent.skills = vec!["dev".into()];
        agent.additional_instructions = Some("end here".into());
        let text = generate(&agent);
        assert!(text.contains("- dev: .opencode/skills/dev/SKILL.md"));
        assert!(text.trim_end().ends_with("end here"));
    }
}
