use super::{EffectiveAgent, GENERATED_BANNER, default_pane, model_id_for};

/// Claude Code agent: YAML frontmatter + markdown body. Deny-only tool
/// model — never writes an allowlist (v1 rule).
pub fn generate(agent: &EffectiveAgent) -> String {
    let source = agent.source;
    let o = &agent.overrides;
    let mut fm = String::new();
    let mut push = |line: String| {
        fm.push_str(&line);
        fm.push('\n');
    };

    push(format!("name: {}", source.name));
    push(format!("description: \"{}\"", escape(&source.description)));
    let model = o.model.as_deref().unwrap_or(&source.model);
    push(format!("model: {}", model_id_for("claude-code", model)));
    let effort = o.effort.as_deref().or(source.effort.as_deref());
    if let Some(effort) = effort.filter(|e| effort_is_real(e)) {
        push(format!("effort: {effort}"));
    }
    let pane = o.pane.unwrap_or_else(|| default_pane(source));
    let background = o.background.unwrap_or(!pane);
    push(format!("background: {background}"));
    if let Some(isolation) = &o.isolation {
        push(format!("isolation: {isolation}"));
    }
    if let Some(memory) = &o.memory {
        push(format!("memory: {memory}"));
    }
    push(format!("disallowedTools: {}", deny_list(agent).join(", ")));
    if let Some(color) = o.color.as_deref().or(source.color.as_deref()) {
        push(format!("color: {color}"));
    }
    if !agent.skills.is_empty() {
        push(format!("skills: {}", agent.skills.join(", ")));
    }
    if !agent.custom_hooks.is_empty() {
        push("hooks:".to_owned());
        for hook in &agent.custom_hooks {
            push(format!("  {}:", hook.event));
            push(format!(
                "    \"{}\":",
                escape(hook.matcher.as_deref().unwrap_or("*"))
            ));
            push("      - type: command".to_owned());
            push(format!("        command: \"{}\"", escape(&hook.command)));
        }
    }

    let mut body = format!("---\n{fm}---\n\n{GENERATED_BANNER}\n\n");
    if let Some(launch) = &agent.launch_instructions {
        body.push_str(&format!("## Launch Instructions\n\n{launch}\n\n"));
    }
    body.push_str(source.body.trim_end());
    body.push('\n');
    if let Some(additional) = &agent.additional_instructions {
        body.push_str(&format!("\n## Additional Instructions\n\n{additional}\n"));
    }
    body
}

/// `Agent` is always denied to subagents; `AskUserQuestion` unless this is
/// the planner; user deny-tools append after.
fn deny_list(agent: &EffectiveAgent) -> Vec<String> {
    let mut deny = vec!["Agent".to_owned()];
    if agent.source.name != "planner" {
        deny.push("AskUserQuestion".to_owned());
    }
    if let Some(extra) = &agent.overrides.deny_tools {
        for tool in extra {
            let tool = claude_tool_name(tool);
            if !deny.contains(&tool) {
                deny.push(tool);
            }
        }
    }
    deny
}

/// v1's alias table: manifests write generic lowercase tool names, Claude
/// matches exact PascalCase — an unmapped name silently fails to deny.
fn claude_tool_name(tool: &str) -> String {
    match tool
        .trim()
        .to_ascii_lowercase()
        .replace(['_', '-'], "")
        .as_str()
    {
        "read" => "Read".into(),
        "grep" => "Grep".into(),
        "glob" | "find" => "Glob".into(),
        "ls" | "list" => "LS".into(),
        "bash" => "Bash".into(),
        "edit" => "Edit".into(),
        "multiedit" => "MultiEdit".into(),
        "write" => "Write".into(),
        "webfetch" => "WebFetch".into(),
        "websearch" => "WebSearch".into(),
        "todowrite" => "TodoWrite".into(),
        "todoread" => "TodoRead".into(),
        "task" | "agent" | "subagent" | "spawnagent" | "spawnagentsoncsv" => "Agent".into(),
        "question" | "askuserquestion" => "AskUserQuestion".into(),
        "notebookread" => "NotebookRead".into(),
        "notebookedit" => "NotebookEdit".into(),
        _ => tool.trim().to_owned(),
    }
}

fn effort_is_real(effort: &str) -> bool {
    !matches!(effort, "" | "none" | "false" | "off" | "no")
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::super::{Role, SourceAgent, parse_source_agent};
    use super::*;
    use crate::manifest::{CustomHook, FrontmatterOverrides, HookAgents};
    use crate::model::{HarnessId, Scope};

    fn engineer() -> SourceAgent {
        parse_source_agent(
            "---\nname: rust\ndescription: Rust \"systems\" engineer\nmodel: opus\nrole: engineer\ncolor: orange\n---\nBody text.\n",
        )
        .unwrap()
    }

    fn effective<'a>(
        source: &'a SourceAgent,
        scope: &'a Scope,
        hooks: Vec<&'a CustomHook>,
    ) -> EffectiveAgent<'a> {
        EffectiveAgent {
            source,
            harness: HarnessId::Claude,
            scope,
            skills: vec!["dev".into(), "rust-perf".into()],
            overrides: FrontmatterOverrides::default(),
            launch_instructions: Some("start here".into()),
            additional_instructions: Some("end here".into()),
            custom_hooks: hooks,
        }
    }

    #[test]
    fn engineer_defaults_pane_true_background_false_and_opus_inherits() {
        let source = engineer();
        let scope = Scope::Global;
        let text = generate(&effective(&source, &scope, vec![]));
        assert!(text.contains("model: inherit"));
        assert!(text.contains("background: false"));
        assert!(text.contains("disallowedTools: Agent, AskUserQuestion"));
        assert!(text.contains("skills: dev, rust-perf"));
        assert!(text.contains("description: \"Rust \\\"systems\\\" engineer\""));
        assert!(text.contains("## Launch Instructions\n\nstart here"));
        assert!(text.trim_end().ends_with("end here"));
        assert!(text.contains("color: orange"));
    }

    #[test]
    fn planner_keeps_questions_and_custom_hooks_render_native() {
        let mut source = engineer();
        source.name = "planner".into();
        source.role = Role::Analyst;
        let scope = Scope::Global;
        let hook = CustomHook {
            event: "PreToolUse".into(),
            matcher: Some("Bash".into()),
            command: "./guard.sh".into(),
            description: None,
            agents: HookAgents::One("all".into()),
        };
        let text = generate(&effective(&source, &scope, vec![&hook]));
        assert!(text.contains("disallowedTools: Agent\n"));
        assert!(!text.contains("AskUserQuestion"));
        assert!(text.contains("hooks:\n  PreToolUse:\n    \"Bash\":"));
        assert!(text.contains("command: \"./guard.sh\""));
        // planner is not an engineer, but the planner pane default applies
        assert!(text.contains("background: false"));
    }

    #[test]
    fn overrides_beat_source_and_deny_tools_append() {
        let source = engineer();
        let scope = Scope::Global;
        let mut agent = effective(&source, &scope, vec![]);
        agent.overrides = FrontmatterOverrides {
            model: Some("sonnet".into()),
            deny_tools: Some(vec!["WebSearch".into()]),
            pane: Some(false),
            color: Some("blue".into()),
            ..FrontmatterOverrides::default()
        };
        let text = generate(&agent);
        assert!(text.contains("model: sonnet"));
        assert!(text.contains("background: true"));
        assert!(text.contains("disallowedTools: Agent, AskUserQuestion, WebSearch"));
        assert!(text.contains("color: blue"));
    }
}
