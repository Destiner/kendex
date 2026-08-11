use vstack_core::apply::Op;
use vstack_core::engine::ops;
use vstack_core::env::Env;

use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

pub fn run(env: &Env, names: Vec<String>, filter: ScopeFilter) -> CliResult {
    if names.is_empty() {
        say("usage: vstack remove <name>… [--scope project|global|all]");
        return Ok(());
    }
    let mut removed_any = false;
    for scope in resolve_scopes(env, filter)? {
        let report = match ops::remove(env, &scope, &names) {
            Ok(report) => report,
            // A scope without a v2 manifest has nothing of ours to remove.
            Err(error) if super::engine_common::is_legacy(&error) => continue,
            Err(error) => return Err(error.into()),
        };
        let touches_artifacts = report
            .plan
            .ops
            .iter()
            .any(|op| matches!(op.op, Op::Trash { .. } | Op::WriteLock { .. }));
        if touches_artifacts {
            removed_any = true;
            vstack_core::apply::execute(env, &report.plan, None)?;
            for op in &report.plan.ops {
                say(&format!("  - {}", op.description));
            }
        }
    }
    if !removed_any {
        say("Nothing removed");
    }
    Ok(())
}
