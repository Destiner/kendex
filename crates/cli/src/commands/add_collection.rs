//! `kendex add https://kendex.ai/c/<id>` — one link, one preview, then:
//! subscribe each repository the scope lacks (at the snapshot commit) and
//! install every member. Existing subscriptions are reused only when
//! their revision matches the snapshot; the steps refuse before anything
//! changes otherwise.

use std::io::{IsTerminal as _, Write as _};

use kendex_core::engine::ops::{self, AddRequest};
use kendex_core::env::Env;
use kendex_core::model::Scope;
use kendex_core::registry::{CurlFetch, collections};
use kendex_core::source_ops::{self, SourceAction};

use super::engine_common::print_report;
use super::{CliResult, say};

pub fn run(env: &Env, scope: &Scope, id: &str, yes: bool) -> CliResult {
    let collection = collections::resolve(&CurlFetch, id)?;
    let steps = source_ops::collection_steps(env, scope, &collection)?;
    say(&format!(
        "collection '{}': {} package(s) across {} repositor{}",
        collection.name,
        collection.members.len(),
        steps.len(),
        if steps.len() == 1 { "y" } else { "ies" }
    ));
    for step in &steps {
        let action = match &step.action {
            SourceAction::Reuse { name } => format!("using existing subscription '{name}'"),
            SourceAction::Subscribe { .. } => match &step.commit {
                Some(commit) => format!("subscribe at {}", &commit[..commit.len().min(7)]),
                None => "subscribe (follows its default branch)".to_owned(),
            },
        };
        let members: Vec<&str> = step
            .agents
            .iter()
            .chain(&step.skills)
            .chain(&step.hooks)
            .chain(&step.commands)
            .chain(&step.mcp_servers)
            .map(String::as_str)
            .collect();
        say(&format!(
            "  {}  [{action}]  {}",
            step.repo,
            members.join(", ")
        ));
    }
    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err("refusing to apply without --yes in a non-interactive session".into());
        }
        let _ = write!(std::io::stderr(), "install all of it? [y/N] ");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            return Err("apply cancelled".into());
        }
    }
    for step in steps {
        let source = match step.action {
            SourceAction::Reuse { name } => name,
            SourceAction::Subscribe { reference } => {
                let subscribed = source_ops::subscribe(env, scope, &reference, None)?;
                kendex_core::apply::execute(env, &subscribed.report.plan, None)?;
                say(&format!(
                    "{}: subscribed to '{}'",
                    scope.label(),
                    subscribed.name
                ));
                subscribed.name
            }
        };
        // The fetch must land before installing from it; the snapshot
        // commit rode in on the subscription's rev.
        if let kendex_core::manifest::ManifestFile::Current(manifest) =
            kendex_core::manifest::load(&kendex_core::manifest::manifest_path(env, scope))?
            && let Some(decl) = manifest.sources.get(&source)
            && let Some(repo) = decl.repo.clone()
        {
            kendex_core::remote::sync(env, &repo, decl.rev.as_deref())?;
        }
        let report = ops::add(
            env,
            scope,
            &AddRequest {
                source: Some(source.clone()),
                agents: step.agents,
                skills: step.skills,
                hooks: step.hooks,
                commands: step.commands,
                mcp_servers: step.mcp_servers,
                pi_extensions: Vec::new(),
                all: false,
                harnesses: None,
                copy: false,
                no_auto_skills: false,
                optional: Vec::new(),
                bundles: Vec::new(),
                hold: false,
            },
        )?;
        print_report(&report);
        kendex_core::apply::execute(env, &report.plan, None)?;
    }
    say("collection installed — every member is in the lock at its resolved commit");
    Ok(())
}
