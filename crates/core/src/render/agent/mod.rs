use serde::{Deserialize, Serialize};

use crate::manifest::{CustomHook, FrontmatterOverrides, HookAgents, Manifest};
use crate::model::{HarnessId, ItemKind, Scope};

pub mod claude;
pub mod codex;
pub mod cursor;
pub mod opencode;
pub mod pi;

/// v1 canonical model tiers translate per harness; exact ids pass through.
pub fn model_id_for(provider: &str, model: &str) -> String {
    let base = model.to_lowercase();
    if base.contains('/') {
        return model.to_owned();
    }
    match provider {
        "openai" => match base.as_str() {
            "opus" | "sonnet" | "haiku" => "openai/gpt-5.6-sol".to_owned(),
            other => format!("openai/{other}"),
        },
        "claude-code" => match base.as_str() {
            "opus" => "inherit".to_owned(),
            other => other.to_owned(),
        },
        _ => base,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Reviewer,
    #[default]
    Engineer,
    Analyst,
    Manager,
}

impl Role {
    pub fn parse(value: &str) -> Option<Role> {
        match value {
            "reviewer" => Some(Role::Reviewer),
            "engineer" => Some(Role::Engineer),
            "analyst" => Some(Role::Analyst),
            "manager" => Some(Role::Manager),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Role::Reviewer => "reviewer",
            Role::Engineer => "engineer",
            Role::Analyst => "analyst",
            Role::Manager => "manager",
        }
    }
}

/// A source agent file: flat YAML frontmatter + markdown body.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceAgent {
    pub name: String,
    pub description: String,
    pub model: String,
    pub role: Role,
    pub color: Option<String>,
    pub effort: Option<String>,
    pub body: String,
}

pub fn parse_source_agent(text: &str) -> Result<SourceAgent, String> {
    let rest = text
        .strip_prefix("---")
        .ok_or("agent file has no frontmatter")?;
    let end = rest.find("\n---").ok_or("unterminated frontmatter")?;
    let mut agent = SourceAgent {
        model: "sonnet".to_owned(),
        ..SourceAgent::default()
    };
    for line in rest[..end].lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'').to_owned();
        match key.trim() {
            "name" => agent.name = value,
            "description" => agent.description = value,
            "model" => agent.model = value,
            "role" => {
                agent.role = Role::parse(&value).ok_or_else(|| {
                    format!("unknown role '{value}' (reviewer|engineer|analyst|manager)")
                })?;
            }
            "color" => agent.color = Some(value),
            "effort" => agent.effort = Some(value),
            _ => {}
        }
    }
    agent.body = rest[end + 4..].trim_start_matches('\n').to_owned();
    if agent.name.is_empty() {
        return Err("agent frontmatter has no name".to_owned());
    }
    Ok(agent)
}

/// v1's pane default: Engineer agents and the planner run in a visible pane.
pub fn default_pane(agent: &SourceAgent) -> bool {
    agent.role == Role::Engineer || agent.name == "planner"
}

/// Everything a per-harness generator needs, already merged.
pub struct EffectiveAgent<'a> {
    pub source: &'a SourceAgent,
    pub harness: HarnessId,
    pub scope: &'a Scope,
    pub skills: Vec<String>,
    pub overrides: FrontmatterOverrides,
    pub launch_instructions: Option<String>,
    pub additional_instructions: Option<String>,
    pub custom_hooks: Vec<&'a CustomHook>,
}

pub const SHARED_START: &str = "<!-- vstack:shared-instructions:start -->";
pub const SHARED_END: &str = "<!-- vstack:shared-instructions:end -->";

/// Shared (`all`/`*`) text renders first inside strippable markers, then the
/// agent-specific text.
pub fn merged_instructions(
    table: &std::collections::BTreeMap<String, String>,
    agent_name: &str,
) -> Option<String> {
    let shared = table.get("all").or_else(|| table.get("*"));
    let specific = table.get(agent_name);
    match (shared, specific) {
        (None, None) => None,
        (None, Some(text)) => Some(text.clone()),
        (Some(shared), specific) => {
            let mut out = format!("{SHARED_START}\n{shared}\n{SHARED_END}");
            if let Some(text) = specific {
                out.push_str("\n\n");
                out.push_str(text);
            }
            Some(out)
        }
    }
}

/// Project overrides win per field over source-side defaults, except
/// deny-tools, which merge (v1 semantics).
pub fn merge_overrides(
    source_defaults: Option<&FrontmatterOverrides>,
    project: Option<&FrontmatterOverrides>,
) -> FrontmatterOverrides {
    let mut merged = source_defaults.cloned().unwrap_or_default();
    let Some(project) = project else {
        return merged;
    };
    macro_rules! take {
        ($field:ident) => {
            if project.$field.is_some() {
                merged.$field = project.$field.clone();
            }
        };
    }
    take!(color);
    take!(model);
    take!(allowed_subagents);
    take!(pane);
    take!(background);
    take!(effort);
    take!(isolation);
    take!(memory);
    take!(mode);
    take!(sandbox_mode);
    take!(model_reasoning_effort);
    take!(nickname_candidates);
    match (&mut merged.deny_tools, &project.deny_tools) {
        (Some(base), Some(extra)) => {
            for tool in extra {
                if !base.contains(tool) {
                    base.push(tool.clone());
                }
            }
        }
        (None, Some(extra)) => merged.deny_tools = Some(extra.clone()),
        _ => {}
    }
    merged
}

pub fn hooks_for_agent<'a>(manifest: &'a Manifest, agent: &SourceAgent) -> Vec<&'a CustomHook> {
    manifest
        .custom_hooks
        .iter()
        .filter(|hook| match &hook.agents {
            HookAgents::One(sel) => sel == "all" || sel == agent.role.name() || sel == &agent.name,
            HookAgents::Many(list) => list
                .iter()
                .any(|sel| sel == &agent.name || sel == agent.role.name()),
        })
        .collect()
}

/// The generated-file banner every harness variant includes.
pub const GENERATED_BANNER: &str = "> Generated by vstack — do not edit; regenerated on every refresh. Intent lives in vstack.toml.";

pub fn generate(agent: &EffectiveAgent) -> String {
    match agent.harness {
        HarnessId::Claude => claude::generate(agent),
        HarnessId::Codex => codex::generate(agent),
        HarnessId::Opencode => opencode::generate(agent),
        HarnessId::Cursor => cursor::generate(agent),
        HarnessId::Pi => pi::generate(agent),
    }
}

/// The filename a generated agent gets in the harness's native dir.
pub fn file_name(harness: HarnessId, agent_name: &str) -> String {
    match harness {
        HarnessId::Codex => format!("{agent_name}.toml"),
        HarnessId::Cursor => format!("{agent_name}.mdc"),
        _ => format!("{agent_name}.md"),
    }
}

/// Skills prose section for harnesses without a native skills field.
pub fn skills_prose(agent: &EffectiveAgent, skill_root_hint: &str) -> Option<String> {
    if agent.skills.is_empty() {
        return None;
    }
    let mut out = String::from("## Required Skills\n\nRead each before acting:\n");
    for skill in &agent.skills {
        out.push_str(&format!("- {skill}: {skill_root_hint}/{skill}/SKILL.md\n"));
    }
    Some(out)
}

/// Custom hooks rendered as prose for harnesses without native hook wiring.
pub fn hooks_prose(agent: &EffectiveAgent) -> Option<String> {
    if agent.custom_hooks.is_empty() {
        return None;
    }
    let mut out = String::new();
    for hook in &agent.custom_hooks {
        out.push_str(&format!(
            "## Safety: {} on {}\n\n{}Run: `{}`\n\n",
            hook.event,
            hook.matcher.as_deref().unwrap_or("every match"),
            hook.description
                .as_ref()
                .map(|d| format!("{d}\n\n"))
                .unwrap_or_default(),
            hook.command
        ));
    }
    Some(out.trim_end().to_owned())
}

pub fn kind() -> ItemKind {
    ItemKind::Agent
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn parses_v1_shaped_agent_frontmatter() {
        let agent = parse_source_agent(
            "---\nname: rust\ndescription: Rust engineer\nmodel: opus\nrole: engineer\neffort: xhigh\ncolor: orange\n---\n\n# Body\n",
        )
        .unwrap();
        assert_eq!(agent.name, "rust");
        assert_eq!(agent.role, Role::Engineer);
        assert_eq!(agent.effort.as_deref(), Some("xhigh"));
        assert!(agent.body.starts_with("# Body"));
        assert!(default_pane(&agent));
    }

    #[test]
    fn shared_instructions_render_first_inside_markers() {
        let mut table = BTreeMap::new();
        table.insert("all".to_owned(), "fleet rule".to_owned());
        table.insert("rust".to_owned(), "rust rule".to_owned());
        let merged = merged_instructions(&table, "rust").unwrap();
        assert!(merged.starts_with(SHARED_START));
        assert!(merged.contains("fleet rule"));
        assert!(merged.ends_with("rust rule"));
        let solo = merged_instructions(&table, "other").unwrap();
        assert!(solo.contains(SHARED_START) && !solo.contains("rust rule"));
    }

    #[test]
    fn deny_tools_merge_while_other_fields_prefer_project() {
        let source = FrontmatterOverrides {
            model: Some("sonnet".into()),
            deny_tools: Some(vec!["WebSearch".into()]),
            ..FrontmatterOverrides::default()
        };
        let project = FrontmatterOverrides {
            model: Some("opus".into()),
            deny_tools: Some(vec!["WebFetch".into(), "WebSearch".into()]),
            ..FrontmatterOverrides::default()
        };
        let merged = merge_overrides(Some(&source), Some(&project));
        assert_eq!(merged.model.as_deref(), Some("opus"));
        assert_eq!(
            merged.deny_tools,
            Some(vec!["WebSearch".into(), "WebFetch".into()])
        );
    }

    #[test]
    fn model_tiers_map_per_provider() {
        assert_eq!(model_id_for("claude-code", "opus"), "inherit");
        assert_eq!(model_id_for("claude-code", "sonnet"), "sonnet");
        assert_eq!(model_id_for("openai", "haiku"), "openai/gpt-5.6-sol");
        assert_eq!(model_id_for("openai", "o9"), "openai/o9");
        assert_eq!(model_id_for("claude-code", "custom/id"), "custom/id");
    }
}
