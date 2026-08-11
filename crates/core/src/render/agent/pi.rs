use super::{EffectiveAgent, GENERATED_BANNER, Role, default_pane, hooks_prose, skills_prose};
use crate::model::Scope;

/// Pi agent: YAML frontmatter + markdown body. Delegation is the whole story
/// here — `allowed-subagents` and `deny-tools` have to agree, so they are
/// resolved together.
pub fn generate(agent: &EffectiveAgent) -> String {
    let source = agent.source;
    let o = &agent.overrides;
    let allowed = allowed_subagents(agent);
    let deny = deny_tools(agent, &allowed);
    let mut out = String::from("---\n");
    out.push_str(&format!("name: {}\n", source.name));
    out.push_str(&format!(
        "description: \"{}\"\n",
        source
            .description
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    ));
    if !deny.is_empty() {
        out.push_str(&format!("deny-tools: {}\n", deny.join(", ")));
    }
    if !allowed.is_empty() {
        out.push_str(&format!("allowed-subagents: {}\n", allowed.join(", ")));
    }
    if let Some(model) = model(agent) {
        out.push_str(&format!("model: {model}\n"));
    }
    if let Some(color) = o.color.as_deref().or(source.color.as_deref()) {
        out.push_str(&format!("color: {color}\n"));
    }
    if o.pane.unwrap_or_else(|| default_pane(source)) {
        out.push_str("pane: true\n");
    }
    out.push_str("---\n\n");
    out.push_str(&body(agent));
    out
}

/// Heavy agents (`opus`) omit `model` so the child inherits the parent
/// session; cheaper tiers pin the codex model with the configured effort.
fn model(agent: &EffectiveAgent) -> Option<String> {
    let effort = agent
        .overrides
        .model_reasoning_effort
        .as_deref()
        .or(agent.overrides.effort.as_deref())
        .or(agent.source.effort.as_deref())
        .filter(|effort| !is_none_value(effort));
    match agent.overrides.model.as_deref() {
        Some(model) if is_inherit(model) => None,
        Some(model) => Some(with_effort(model, effort)),
        None => match agent.source.model.to_lowercase().as_str() {
            "opus" => None,
            "sonnet" | "haiku" => Some(with_effort("sonnet", effort)),
            other => Some(other.to_owned()),
        },
    }
}

fn with_effort(model: &str, effort: Option<&str>) -> String {
    match model.to_lowercase().as_str() {
        "opus" | "sonnet" | "haiku" => {
            let suffix = effort.map(|e| format!(":{e}")).unwrap_or_default();
            format!("openai-codex/gpt-5.6-sol{suffix}")
        }
        other => other.to_owned(),
    }
}

fn is_inherit(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "inherit" | "current" | "parent"
    )
}

/// Engineers delegate reconnaissance to scout by default; every other role
/// stays a leaf.
fn allowed_subagents(agent: &EffectiveAgent) -> Vec<String> {
    let list = match &agent.overrides.allowed_subagents {
        Some(list) => list.clone(),
        None if agent.source.role == Role::Engineer => vec!["scout".to_owned()],
        None => Vec::new(),
    };
    let mut out: Vec<String> = Vec::new();
    for name in list {
        let name = name.trim().to_owned();
        if name.is_empty() || out.iter().any(|kept| kept.eq_ignore_ascii_case(&name)) {
            continue;
        }
        out.push(name);
    }
    out
}

fn deny_tools(agent: &EffectiveAgent, allowed: &[String]) -> Vec<String> {
    let user = agent.overrides.deny_tools.as_deref().unwrap_or(&[]);
    let mut tools: Vec<String> = [
        "subagent",
        "get_subagent_result",
        "steer_subagent",
        "stop_subagent",
    ]
    .iter()
    .map(|tool| (*tool).to_owned())
    .collect();
    if allowed.is_empty() {
        tools.push("delegate_subagent".to_owned());
    }
    if agent.source.name != "planner" {
        tools.push("question".to_owned());
    }
    if agent.source.role == Role::Reviewer {
        tools.push("tasks_write".to_owned());
    }
    tools.extend(user.iter().cloned());

    let mut out: Vec<String> = Vec::new();
    for tool in tools {
        if tool.trim().is_empty() || out.iter().any(|kept| normalize(kept) == normalize(&tool)) {
            continue;
        }
        out.push(tool);
    }
    // A live allowlist needs the delegation tool, so the default deny goes —
    // unless the user asked for it, in which case their policy wins and the
    // allowlist stays inert.
    let user_denies_delegate = user
        .iter()
        .any(|tool| normalize(tool) == "delegate_subagent");
    if !allowed.is_empty() && !user_denies_delegate {
        out.retain(|tool| normalize(tool) != "delegate_subagent");
    }
    out
}

fn normalize(tool: &str) -> String {
    tool.trim().to_lowercase().replace('-', "_")
}

fn body(agent: &EffectiveAgent) -> String {
    let mut out = format!("{GENERATED_BANNER}\n\n");
    if let Some(launch) = &agent.launch_instructions {
        out.push_str(&format!("## Launch Instructions\n\n{launch}\n\n"));
    }
    out.push_str(agent.source.body.trim_end());
    out.push('\n');
    let skill_root = match agent.scope {
        Scope::Global => "~/.pi/agent/skills",
        Scope::Project { .. } => ".agents/skills",
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
    use crate::manifest::FrontmatterOverrides;
    use crate::model::HarnessId;

    fn source(name: &str, role: &str, model: &str) -> SourceAgent {
        parse_source_agent(&format!(
            "---\nname: {name}\ndescription: Pi agent\nmodel: {model}\nrole: {role}\ncolor: green\n---\nBody text.\n"
        ))
        .unwrap()
    }

    fn effective<'a>(source: &'a SourceAgent, scope: &'a Scope) -> EffectiveAgent<'a> {
        EffectiveAgent {
            source,
            harness: HarnessId::Pi,
            scope,
            skills: vec![],
            overrides: FrontmatterOverrides::default(),
            launch_instructions: None,
            additional_instructions: None,
            custom_hooks: vec![],
        }
    }

    fn deny_line(text: &str) -> String {
        text.lines()
            .find(|line| line.starts_with("deny-tools:"))
            .unwrap_or_default()
            .to_owned()
    }

    #[test]
    fn engineer_keeps_scout_delegation_and_inherits_the_opus_model() {
        let source = source("rust", "engineer", "opus");
        let scope = Scope::Global;
        let text = generate(&effective(&source, &scope));
        assert!(text.contains("allowed-subagents: scout\n"));
        assert!(text.contains("pane: true\n"));
        assert!(text.contains("color: green\n"));
        assert!(!text.lines().any(|line| line.starts_with("model:")));
        assert_eq!(
            deny_line(&text),
            "deny-tools: subagent, get_subagent_result, steer_subagent, stop_subagent, question"
        );
    }

    #[test]
    fn reviewer_loses_delegation_and_task_writes_and_pins_the_codex_model() {
        let mut source = source("reviewer-arch", "reviewer", "sonnet");
        source.effort = Some("high".into());
        let scope = Scope::Global;
        let text = generate(&effective(&source, &scope));
        assert!(text.contains("model: openai-codex/gpt-5.6-sol:high\n"));
        assert!(!text.contains("allowed-subagents:"));
        assert!(!text.contains("pane: true"));
        assert_eq!(
            deny_line(&text),
            "deny-tools: subagent, get_subagent_result, steer_subagent, stop_subagent, delegate_subagent, question, tasks_write"
        );
    }

    #[test]
    fn an_explicit_delegate_deny_survives_a_live_allowlist() {
        let source = source("rust", "engineer", "opus");
        let scope = Scope::Global;
        let mut agent = effective(&source, &scope);
        agent.overrides = FrontmatterOverrides {
            deny_tools: Some(vec!["delegate-subagent".into()]),
            ..FrontmatterOverrides::default()
        };
        let text = generate(&agent);
        assert!(deny_line(&text).contains("delegate-subagent"));
        assert!(text.contains("allowed-subagents: scout\n"));

        agent.overrides = FrontmatterOverrides {
            allowed_subagents: Some(vec![]),
            ..FrontmatterOverrides::default()
        };
        let text = generate(&agent);
        assert!(!text.contains("allowed-subagents:"));
        assert!(deny_line(&text).contains("delegate_subagent"));
    }

    #[test]
    fn planner_keeps_questions_and_overrides_win_over_source() {
        let source = source("planner", "analyst", "opus");
        let scope = Scope::Project {
            root: "/tmp/proj".into(),
        };
        let mut agent = effective(&source, &scope);
        agent.skills = vec!["dev".into()];
        agent.overrides = FrontmatterOverrides {
            model: Some("inherit".into()),
            allowed_subagents: Some(vec!["scout".into(), " Scout ".into(), "researcher".into()]),
            color: Some("magenta".into()),
            pane: Some(false),
            ..FrontmatterOverrides::default()
        };
        let text = generate(&agent);
        assert!(!deny_line(&text).contains("question"));
        assert!(text.contains("allowed-subagents: scout, researcher\n"));
        assert!(text.contains("color: magenta\n"));
        assert!(!text.contains("pane: true"));
        assert!(!text.lines().any(|line| line.starts_with("model:")));
        assert!(text.contains("- dev: .agents/skills/dev/SKILL.md"));
    }
}
