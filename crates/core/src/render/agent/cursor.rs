use super::{EffectiveAgent, GENERATED_BANNER};

/// Cursor has no agents — an agent installs as a rule file. Rules carry no
/// model, tool, skill or hook fields, so only the prompt survives.
pub fn generate(agent: &EffectiveAgent) -> String {
    let source = agent.source;
    let mut out = String::from("---\n");
    out.push_str(&format!(
        "description: \"{} — {}\"\n",
        escape(&source.name),
        escape(&source.description)
    ));
    out.push_str("alwaysApply: false\n---\n\n");
    out.push_str(&format!("{GENERATED_BANNER}\n\n"));
    if let Some(launch) = &agent.launch_instructions {
        out.push_str(&format!("## Launch Instructions\n\n{launch}\n\n"));
    }
    out.push_str(source.body.trim_end());
    out.push('\n');
    if let Some(additional) = &agent.additional_instructions {
        out.push_str(&format!("\n## Additional Instructions\n\n{additional}\n"));
    }
    out
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAgent, parse_source_agent};
    use super::*;
    use crate::manifest::{CustomHook, FrontmatterOverrides, HookAgents};
    use crate::model::{HarnessId, Scope};

    fn source() -> SourceAgent {
        parse_source_agent(
            "---\nname: rust\ndescription: Rust engineer\nmodel: opus\nrole: engineer\ncolor: orange\n---\nBody text.\n",
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
            harness: HarnessId::Cursor,
            scope,
            skills: vec!["dev".into()],
            overrides: FrontmatterOverrides {
                model: Some("sonnet".into()),
                color: Some("blue".into()),
                ..FrontmatterOverrides::default()
            },
            launch_instructions: Some("start here".into()),
            additional_instructions: Some("end here".into()),
            custom_hooks: hooks,
        }
    }

    #[test]
    fn frontmatter_is_only_a_description_and_always_apply_false() {
        let source = source();
        let scope = Scope::Project {
            root: "/tmp/proj".into(),
        };
        let text = generate(&effective(&source, &scope, vec![]));
        assert!(
            text.starts_with(
                "---\ndescription: \"rust — Rust engineer\"\nalwaysApply: false\n---\n"
            )
        );
        assert!(!text.contains("model:"));
        assert!(!text.contains("color:"));
    }

    #[test]
    fn skills_and_hooks_are_dropped_but_instructions_survive() {
        let source = source();
        let scope = Scope::Project {
            root: "/tmp/proj".into(),
        };
        let hook = CustomHook {
            event: "PreToolUse".into(),
            matcher: Some("Bash".into()),
            command: "./guard.sh".into(),
            description: None,
            agents: HookAgents::One("all".into()),
        };
        let text = generate(&effective(&source, &scope, vec![&hook]));
        assert!(!text.contains("Required Skills"));
        assert!(!text.contains("guard.sh"));
        assert!(text.contains("## Launch Instructions\n\nstart here"));
        assert!(text.contains("Body text."));
        assert!(text.trim_end().ends_with("end here"));
    }
}
