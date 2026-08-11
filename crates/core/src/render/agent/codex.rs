use super::{EffectiveAgent, GENERATED_BANNER, Role, hooks_prose, skills_prose};
use crate::model::Scope;

const NICKNAME_SUFFIXES: [&str; 6] = ["Atlas", "Delta", "Echo", "Nova", "Orion", "Vector"];

/// Codex agent: TOML whose `developer_instructions` carries the whole prompt.
/// No native skills field and no hook wiring, so both render as prose.
pub fn generate(agent: &EffectiveAgent) -> String {
    let source = agent.source;
    let o = &agent.overrides;
    let mut out = String::new();
    out.push_str(&format!("name = \"{}\"\n", escape(&source.name)));
    out.push_str(&format!(
        "nickname_candidates = [{}]\n",
        nicknames(agent)
            .iter()
            .map(|n| format!("\"{}\"", escape(n)))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(&format!(
        "description = \"{}\"\n",
        escape(&source.description)
    ));
    let model = o.model.as_deref().unwrap_or(&source.model);
    out.push_str(&format!("model = \"{}\"\n", codex_model(model)));
    let effort = o
        .model_reasoning_effort
        .as_deref()
        .or(o.effort.as_deref())
        .or(source.effort.as_deref())
        .filter(|effort| !is_none_value(effort));
    if let Some(effort) = effort {
        out.push_str(&format!("model_reasoning_effort = \"{effort}\"\n"));
    }
    out.push_str(&format!("sandbox_mode = \"{}\"\n", sandbox_mode(agent)));
    out.push_str("developer_instructions = '''\n");
    out.push_str(&fence_safe(&instructions(agent)));
    out.push_str("'''\n");
    out
}

/// Engineers need to reach outside the workspace; everyone else writes only
/// report artifacts, which workspace-write already allows.
fn sandbox_mode(agent: &EffectiveAgent) -> String {
    if let Some(mode) = &agent.overrides.sandbox_mode {
        return mode.clone();
    }
    match agent.source.role {
        Role::Engineer => "danger-full-access".to_owned(),
        Role::Analyst | Role::Reviewer | Role::Manager => "workspace-write".to_owned(),
    }
}

fn codex_model(model: &str) -> String {
    match model.to_lowercase().as_str() {
        "opus" | "sonnet" | "haiku" => "gpt-5.6-sol".to_owned(),
        other => other.to_owned(),
    }
}

fn nicknames(agent: &EffectiveAgent) -> Vec<String> {
    let custom: Vec<String> = agent
        .overrides
        .nickname_candidates
        .iter()
        .flatten()
        .map(|candidate| candidate.trim().to_owned())
        .filter(|candidate| !candidate.is_empty())
        .collect();
    if !custom.is_empty() {
        return custom;
    }
    let prefix = display_name(&agent.source.name);
    NICKNAME_SUFFIXES
        .iter()
        .map(|suffix| format!("{prefix}-{suffix}"))
        .collect()
}

fn display_name(name: &str) -> String {
    let parts: Vec<String> = name
        .trim()
        .split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect();
    if parts.is_empty() {
        return "Agent".to_owned();
    }
    parts.join("-")
}

fn capitalize(part: &str) -> String {
    if part.eq_ignore_ascii_case("tpm") {
        return "TPM".to_owned();
    }
    let mut chars = part.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

fn instructions(agent: &EffectiveAgent) -> String {
    let mut out = format!("{GENERATED_BANNER}\n\n");
    if let Some(launch) = &agent.launch_instructions {
        out.push_str(&format!("## Launch Instructions\n\n{launch}\n\n"));
    }
    out.push_str(agent.source.body.trim_end());
    out.push('\n');
    let skill_root = match agent.scope {
        Scope::Global => "$CODEX_HOME/skills",
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

/// TOML literal strings have no escape mechanism, so an apostrophe run in the
/// prompt would close the block early. Break every run down to two.
fn fence_safe(text: &str) -> String {
    if text.contains("'''") {
        return text.replace("''", "' '");
    }
    text.to_owned()
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
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

    fn source(name: &str, role: &str) -> SourceAgent {
        parse_source_agent(&format!(
            "---\nname: {name}\ndescription: Codex agent\nmodel: sonnet\nrole: {role}\n---\nBody text.\n"
        ))
        .unwrap()
    }

    fn effective<'a>(source: &'a SourceAgent, scope: &'a Scope) -> EffectiveAgent<'a> {
        EffectiveAgent {
            source,
            harness: HarnessId::Codex,
            scope,
            skills: vec!["dev".into()],
            overrides: FrontmatterOverrides::default(),
            launch_instructions: None,
            additional_instructions: None,
            custom_hooks: vec![],
        }
    }

    #[test]
    fn engineer_gets_full_access_and_others_workspace_write() {
        let engineer = source("rust", "engineer");
        let manager = source("tpm", "manager");
        let scope = Scope::Project {
            root: "/tmp/proj".into(),
        };
        let engineer = generate(&effective(&engineer, &scope));
        let manager = generate(&effective(&manager, &scope));
        assert!(engineer.contains("sandbox_mode = \"danger-full-access\""));
        assert!(manager.contains("sandbox_mode = \"workspace-write\""));
        assert!(engineer.contains("model = \"gpt-5.6-sol\""));
        assert!(!engineer.contains("model_reasoning_effort"));
        assert!(engineer.contains("- dev: .agents/skills/dev/SKILL.md"));
    }

    #[test]
    fn nicknames_capitalize_each_part_and_keep_known_acronyms() {
        let reviewer = source("reviewer-arch", "reviewer");
        let tpm = source("tpm", "manager");
        let scope = Scope::Global;
        let reviewer = generate(&effective(&reviewer, &scope));
        let tpm = generate(&effective(&tpm, &scope));
        assert!(reviewer.contains(
            "nickname_candidates = [\"Reviewer-Arch-Atlas\", \"Reviewer-Arch-Delta\", \"Reviewer-Arch-Echo\", \"Reviewer-Arch-Nova\", \"Reviewer-Arch-Orion\", \"Reviewer-Arch-Vector\"]"
        ));
        assert!(tpm.contains("\"TPM-Atlas\""));
        assert!(tpm.contains("- dev: $CODEX_HOME/skills/dev/SKILL.md"));
    }

    #[test]
    fn overrides_replace_sandbox_model_effort_and_nicknames() {
        let source = source("rust", "engineer");
        let scope = Scope::Global;
        let mut agent = effective(&source, &scope);
        agent.overrides = FrontmatterOverrides {
            sandbox_mode: Some("read-only".into()),
            model: Some("o9-preview".into()),
            effort: Some("xhigh".into()),
            nickname_candidates: Some(vec!["Rust-One".into(), " ".into()]),
            ..FrontmatterOverrides::default()
        };
        let text = generate(&agent);
        assert!(text.contains("sandbox_mode = \"read-only\""));
        assert!(text.contains("model = \"o9-preview\""));
        assert!(text.contains("model_reasoning_effort = \"xhigh\""));
        assert!(text.contains("nickname_candidates = [\"Rust-One\"]"));
    }

    #[test]
    fn apostrophe_runs_never_close_the_instruction_block() {
        let mut source = source("rust", "engineer");
        source.body = "Use ''' fences sparingly.".into();
        let scope = Scope::Global;
        let text = generate(&effective(&source, &scope));
        assert_eq!(text.matches("'''").count(), 2);
        assert!(text.ends_with("'''\n"));
    }
}
