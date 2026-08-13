use vstack_core::engine::plan_refresh;
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
pub fn run(env: &Env, filter: ScopeFilter, verbose: bool, yes: bool) -> CliResult {
    let mut refreshed_anything = false;
    let mut failures: Vec<String> = Vec::new();

    for scope in resolve_scopes(env, filter)? {
        let manifest_path = vstack_core::manifest::manifest_path(env, &scope);
        if let Ok(vstack_core::manifest::ManifestFile::Current(manifest)) =
            vstack_core::manifest::load(&manifest_path)
        {
            match vstack_core::remote::sync_sources(env, &manifest) {
                Ok(warnings) => {
                    for warning in warnings {
                        say(&format!("warning: {warning}"));
                    }
                }
                Err(error) => failures.push(error.to_string()),
            }
        }
        let report = match plan_refresh(env, &scope) {
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
