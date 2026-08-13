use super::{EffectiveAgent, GENERATED_BANNER, RenderedAgent};
use crate::render::permission::PermissionIntent;

/// Cursor has no agents — an agent installs as a rule file. Rules carry no
/// model, tool, skill or hook fields, so only the prompt survives. A rule
/// grants no tools, so permission intent is advisory here, not widened —
/// but the user should hear that nothing enforces it.
pub fn generate(agent: &EffectiveAgent) -> RenderedAgent {
    let mut warnings = Vec::new();
    if !matches!(agent.permissions, PermissionIntent::Unspecified) {
        warnings.push(
            "Cursor rules carry no tool permissions — this agent's tool restrictions are advisory text only"
                .to_owned(),
        );
    }
    let source = agent.source;
    let mut out = String::from("---\n");
    out.push_str(&format!(
        "description: {}\n",
        crate::render::yaml_quoted(&format!("{} — {}", source.name, source.description))
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
    RenderedAgent {
        text: out,
        warnings,
    }
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
            permissions: PermissionIntent::Unspecified,
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
        let text = generate(&effective(&source, &scope, vec![])).text;
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
        let text = generate(&effective(&source, &scope, vec![&hook])).text;
        assert!(!text.contains("Required Skills"));
        assert!(!text.contains("guard.sh"));
        assert!(text.contains("## Launch Instructions\n\nstart here"));
        assert!(text.contains("Body text."));
        assert!(text.trim_end().ends_with("end here"));
    }

    #[test]
    fn permission_intent_warns_that_cursor_cannot_enforce_it() {
        let source = source();
        let scope = Scope::Project {
            root: "/tmp/proj".into(),
        };
        let mut agent = effective(&source, &scope, vec![]);
        agent.permissions = PermissionIntent::allow_only(vec!["Read".into()]);
        let rendered = generate(&agent);
        assert!(rendered.warnings.iter().any(|w| w.contains("advisory")));
    }
}
