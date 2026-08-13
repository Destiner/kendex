use std::io::{IsTerminal, Write};

use vstack_core::apply::Op;
use vstack_core::engine::{EngineReport, ops};
use vstack_core::env::Env;
use vstack_core::model::Scope;

use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

/// `sweep` is the answer to "and the things only these items needed?" —
/// `None` means nobody has answered yet.
pub fn run(env: &Env, names: Vec<String>, filter: ScopeFilter, sweep: Option<bool>) -> CliResult {
    if names.is_empty() {
        say("usage: vstack remove <name>… [--scope project|global|all]");
        return Ok(());
    }
    let mut removed_any = false;
    for scope in resolve_scopes(env, filter)? {
        let report = match ops::remove(env, &scope, &names, sweep.unwrap_or(false)) {
            Ok(report) => report,
            // A scope without a v2 manifest has nothing of ours to remove.
            Err(error) if super::engine_common::is_legacy(&error) => continue,
            Err(error) => return Err(error.into()),
        };
        let report = match answer(env, &scope, &names, report, sweep)? {
            Some(report) => report,
            None => continue,
        };
        // What still wants a removed item says so now, not on the next audit.
        for warning in &report.warnings {
            say(&format!("warning: {}: {}", warning.name, warning.message));
        }
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

/// Removing the last thing that needed something leaves it behind. Asking is
/// the whole point of the step, so with nobody to ask — no terminal and no
/// flag — the removal stops before it writes anything, naming the flags that
/// answer it.
fn answer(
    env: &Env,
    scope: &Scope,
    names: &[String],
    report: EngineReport,
    sweep: Option<bool>,
) -> Result<Option<EngineReport>, Box<dyn std::error::Error>> {
    if sweep.is_some() || report.sweepable.is_empty() {
        return Ok(Some(report));
    }
    let leftovers: Vec<String> = report
        .sweepable
        .iter()
        .map(|change| format!("{} {}", change.kind.name(), change.name))
        .collect();
    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "removing this leaves {} behind that nothing needs anymore — pass --sweep to remove them too, or --no-sweep to keep them",
            leftovers.join(", ")
        )
        .into());
    }
    let _ = write!(
        std::io::stderr(),
        "also remove {}, which nothing needs anymore? [y/N] ",
        leftovers.join(", ")
    );
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    match matches!(answer.trim(), "y" | "Y" | "yes") {
        true => Ok(Some(ops::remove(env, scope, names, true)?)),
        false => Ok(Some(report)),
    }
}
