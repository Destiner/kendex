use vstack_core::engine::{PlanOptions, plan_apply};
use vstack_core::env::Env;
use vstack_core::lock::{load as load_lock, lock_path};

use super::engine_common::{confirm_and_execute, refresh_failures};
use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

/// Regenerate every declared installation, and re-derive what those
/// declarations pull in — a dependency that appeared upstream, one that went
/// away. Regenerating is automatic; changing *what is installed* is shown
/// first and needs an answer. Orphans nobody derived are left alone, as in
/// v1: `remove` and `apply` clean those up.
#[derive(clap::Args)]
pub struct RefreshArgs {
    #[arg(short = 'g', long)]
    global: bool,
    /// project | global | all (default all)
    #[arg(long)]
    scope: Option<String>,
    /// Per-item detail instead of the compact summary
    #[arg(short = 'v', long)]
    verbose: bool,
    /// Accept changes to what is installed without asking
    #[arg(short = 'y', long)]
    yes: bool,
    /// Overwrite installations you edited by hand
    #[arg(long)]
    discard_edits: bool,
}

pub fn run_args(env: &Env, args: RefreshArgs) -> CliResult {
    let filter = ScopeFilter::resolve(args.scope.as_deref(), args.global, ScopeFilter::All)?;
    run(env, filter, args.verbose, args.yes, args.discard_edits)
}

pub fn run(
    env: &Env,
    filter: ScopeFilter,
    verbose: bool,
    yes: bool,
    discard_edits: bool,
) -> CliResult {
    let mut refreshed_anything = false;
    let mut failures: Vec<String> = Vec::new();

    for scope in resolve_scopes(env, filter)? {
        let manifest_path = vstack_core::manifest::manifest_path(env, &scope);
        if let Ok(vstack_core::manifest::ManifestFile::Current(manifest)) =
            vstack_core::manifest::load(&manifest_path)
        {
            // An unreachable catalog is reported, not fatal: what came from
            // every other catalog still refreshes.
            for note in vstack_core::remote::sync_declared_sources(env, &manifest) {
                say(&format!("warning: {note}"));
            }
        }
        let options = PlanOptions {
            sweep_unneeded: true,
            overwrite_edited: discard_edits,
            ..PlanOptions::default()
        };
        let report = match plan_apply(env, &scope, &options) {
            Ok(report) => report,
            Err(error) => {
                failures.push(error.to_string());
                continue;
            }
        };
        let lock = load_lock(&lock_path(env, &scope))?;
        if lock.entries.is_empty() && report.plan.is_empty() {
            continue;
        }
        refreshed_anything = true;
        failures.extend(refresh_failures(&report));
        if verbose {
            for row in &report.drift {
                say(&format!(
                    "{} {} [{}]: {:?} — {}",
                    row.kind.name(),
                    row.name,
                    row.harness.name(),
                    row.state,
                    row.detail
                ));
            }
        }
        if report.plan.is_empty() {
            say(&format!("{}: up to date", scope.label()));
            continue;
        }
        if !report.set_changes.is_empty() {
            say(&format!(
                "{}: this changes what is installed",
                scope.label()
            ));
            for change in &report.set_changes {
                let verb = match change.direction {
                    vstack_core::engine::SetDirection::Add => "install",
                    vstack_core::engine::SetDirection::Remove => "remove",
                };
                say(&format!(
                    "  - {verb} {} {} for {} — {}",
                    change.kind.name(),
                    change.name,
                    change.harness.display_name(),
                    change.reason
                ));
            }
            if let Err(error) = confirm_and_execute(env, &report, yes) {
                failures.push(error.to_string());
            }
            continue;
        }
        match vstack_core::apply::execute(env, &report.plan, None) {
            Ok(outcome) => say(&format!(
                "{}: refreshed {} change(s)",
                scope.label(),
                outcome.applied
            )),
            Err(error) => failures.push(error.to_string()),
        }
    }

    if !refreshed_anything && failures.is_empty() {
        say("nothing installed");
        return Ok(());
    }
    if !failures.is_empty() {
        for failure in &failures {
            say(&format!("failed: {failure}"));
        }
        return Err(format!("failed to refresh {} item/source(s)", failures.len()).into());
    }
    Ok(())
}
