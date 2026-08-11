use std::collections::BTreeMap;

use vstack_core::env::Env;
use vstack_core::model::Scope;
use vstack_core::{scan, settings};

use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

/// Phase 1 check is detection sanity: which harnesses exist, what was
/// observed where, and every surface the scan could not read.
pub fn run(env: &Env, filter: ScopeFilter) -> CliResult {
    say(&format!("vstack {}", env!("CARGO_PKG_VERSION")));
    let scopes = resolve_scopes(env, filter)?;
    let app_settings = settings::load(env)?;
    let result = scan::scan_scopes(env, &app_settings.harness_roots, &scopes);

    if result.harnesses.is_empty() {
        say("no harnesses detected");
    }
    for harness in &result.harnesses {
        say(&format!(
            "{}: {}",
            harness.harness.name(),
            harness.root.display()
        ));
    }

    let mut counts: BTreeMap<(String, &str), usize> = BTreeMap::new();
    for item in &result.items {
        let scope = match &item.scope {
            Scope::Global => "global".to_owned(),
            Scope::Project { root } => root.display().to_string(),
        };
        *counts.entry((scope, item.kind.name())).or_default() += 1;
    }
    for ((scope, kind), count) in &counts {
        say(&format!("{scope}: {count} {kind}(s)"));
    }

    for warning in &result.warnings {
        say(&format!("warning: {warning}"));
    }

    // v1 contract: drift alone is informational; an agent referencing a
    // skill that is not declared is the one hard failure.
    let mut missing_refs = 0usize;
    for scope in &scopes {
        let path = vstack_core::manifest::manifest_path(env, scope);
        let Ok(vstack_core::manifest::ManifestFile::Current(manifest)) =
            vstack_core::manifest::load(&path)
        else {
            continue;
        };
        for (agent, skills) in &manifest.agent_skills {
            for skill in skills {
                if !manifest.skills.contains_key(skill) {
                    missing_refs += 1;
                    say(&format!(
                        "! {}: agent '{agent}' references skill '{skill}' which is not declared",
                        scope.label()
                    ));
                }
            }
        }
    }
    if missing_refs > 0 {
        return Err(format!("{missing_refs} skill reference(s) missing from install").into());
    }
    Ok(())
}
