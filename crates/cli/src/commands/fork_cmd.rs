use clap::Args;

use vstack_core::engine::audit;
use vstack_core::env::Env;
use vstack_core::model::HarnessId;

use super::pin::parse_kind;
use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

#[derive(Args)]
pub struct ForkArgs {
    /// agent | skill
    kind: String,
    name: String,
    /// Rename an existing fork to this name instead of forking
    #[arg(long)]
    rename: Option<String>,
    /// Which tool's rendering holds the edit (agents; default claude)
    #[arg(long)]
    harness: Option<String>,
    #[arg(short = 'g', long)]
    global: bool,
    /// project | global (default project)
    #[arg(long)]
    scope: Option<String>,
}

pub fn run(env: &Env, args: ForkArgs) -> CliResult {
    let kind = parse_kind(&args.kind)?;
    let harness = match &args.harness {
        Some(value) => {
            HarnessId::parse(value).ok_or_else(|| format!("unknown harness '{value}'"))?
        }
        None => HarnessId::Claude,
    };
    let filter = ScopeFilter::resolve(args.scope.as_deref(), args.global, ScopeFilter::Project)?;
    let scope = resolve_scopes(env, filter)?.remove(0);

    let plan = match &args.rename {
        Some(new) => vstack_core::engine::fork::rename_fork(env, &scope, kind, &args.name, new)?,
        None => vstack_core::engine::fork::fork(env, &scope, kind, &args.name, harness)?,
    };
    for op in &plan.ops {
        say(&format!("  - {}", op.description));
    }
    vstack_core::apply::execute(env, &plan, None)?;

    // Second transaction renders the fork (or the renamed fork) in place.
    let report = audit(env, &scope)?;
    vstack_core::apply::execute(env, &report.plan, None)?;
    match args.rename {
        Some(new) => say(&format!("fork renamed to {new}")),
        None => say(&format!(
            "{} '{}' is yours now — a local fork, updates paused",
            kind.name(),
            args.name
        )),
    }
    Ok(())
}
