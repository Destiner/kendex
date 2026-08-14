use vstack_core::engine::{PlanOptions, plan_scope};
use vstack_core::env::Env;
use vstack_core::lock::{load as load_lock, lock_path};
use vstack_core::manifest;

use super::engine_common::{confirm_and_execute, print_report};
use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

/// Make disk match declaration — orphan cleanup included, plan shown first.
/// (Repurposed from v1's theme-pack apply; extras are gone in v2.)
///
/// `allow_unsafe` names items whose safety findings the user has read and
/// accepted. Nothing is granted in advance: the review is recorded against
/// the exact content and findings this run produced, by the same plan that
/// installs them, so it expires the moment either one changes.
pub fn run(
    env: &Env,
    filter: ScopeFilter,
    plan_only: bool,
    yes: bool,
    allow_unsafe: Vec<String>,
) -> CliResult {
    for scope in resolve_scopes(env, filter)? {
        let path = manifest::manifest_path(env, &scope);
        let Some(loaded) = manifest::load_for_mutation(&path)? else {
            say(&format!("{}: no manifest", scope.label()));
            continue;
        };
        let lock = load_lock(&lock_path(env, &scope))?;
        let options = PlanOptions {
            remove_orphans: true,
            removal_filter: None,
            allow_unsafe: allow_unsafe.clone(),
            ..PlanOptions::default()
        };
        let report = plan_scope(env, &scope, &loaded, &lock, &options)?;
        say(&format!("{}:", scope.label()));
        print_report(&report);
        if !plan_only {
            confirm_and_execute(env, &report, yes)?;
        }
    }
    Ok(())
}
