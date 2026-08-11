use vstack_core::engine::audit;
use vstack_core::env::Env;
use vstack_core::lock::{load as load_lock, lock_path};

use super::engine_common::refresh_failures;
use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

/// Regenerate every declared installation. Orphans are untouched (v1
/// semantics — `remove` and `apply` clean those up).
pub fn run(env: &Env, filter: ScopeFilter, verbose: bool) -> CliResult {
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
        let report = match audit(env, &scope) {
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
