use std::io::{IsTerminal, Write};

use vstack_core::engine::EngineReport;
use vstack_core::env::Env;
use vstack_core::error::CoreError;
use vstack_core::model::HarnessId;

use super::{CliResult, say};

pub fn parse_harnesses(values: &[String]) -> Result<Vec<HarnessId>, String> {
    values
        .iter()
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| HarnessId::parse(v).ok_or(format!("unknown harness '{v}'")))
        .collect()
}

pub fn print_report(report: &EngineReport) {
    for note in &report.notes {
        say(&format!("note: {note}"));
    }
    for warning in &report.warnings {
        let target = match warning.harness {
            Some(harness) => format!("{} ({})", warning.name, harness.display_name()),
            None => warning.name.clone(),
        };
        say(&format!("warning: {target}: {}", warning.message));
        if let Some(fix) = &warning.remediation {
            say(&format!("  fix: {fix}"));
        }
    }
    print_safety(report);
    if report.plan.is_empty() {
        say("nothing to do");
        return;
    }
    say("plan:");
    for op in &report.plan.ops {
        say(&format!("  - {}", op.description));
    }
}

/// What the safety rules found in the content this plan would write. Held
/// back items come first: they are the ones nothing will install.
pub fn print_safety(report: &EngineReport) {
    let mut rows: Vec<&vstack_core::engine::ItemSafety> = report
        .safety
        .iter()
        .filter(|row| !row.findings.is_empty())
        .collect();
    rows.sort_by_key(|row| (!row.blocked(), row.safety.score));
    for row in rows {
        let held = match row.blocked() {
            true => " — held back, nothing will be installed",
            false => "",
        };
        say(&format!(
            "safety: {} {} for {} scores {}/100{held}",
            row.kind.name(),
            row.name,
            row.harness.display_name(),
            row.safety.score
        ));
        for finding in &row.findings {
            say(&format!(
                "  [{}] {}: {}",
                finding.severity.name(),
                finding.location,
                finding.message
            ));
            say(&format!("    fix: {}", finding.remediation));
        }
        if row.blocked() {
            say(&format!(
                "    to install it anyway, review the findings and re-run with --allow-unsafe {}",
                row.name
            ));
        }
    }
}

/// Prompted apply: `--yes` skips the prompt; a non-tty without `--yes`
/// refuses rather than guessing.
pub fn confirm_and_execute(env: &Env, report: &EngineReport, yes: bool) -> CliResult {
    if report.plan.is_empty() {
        return Ok(());
    }
    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err("refusing to apply without --yes in a non-interactive session".into());
        }
        let _ = write!(std::io::stderr(), "apply? [y/N] ");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            return Err("apply cancelled".into());
        }
    }
    let outcome = vstack_core::apply::execute(env, &report.plan, None)?;
    say(&format!("applied {} change(s)", outcome.applied));
    Ok(())
}

/// A refresh failure per v1: any per-item failure or a locked item missing
/// from its source is a hard error.
pub fn refresh_failures(report: &EngineReport) -> Vec<String> {
    report
        .notes
        .iter()
        .filter(|n| {
            n.contains("not found in source")
                || n.contains("missing at")
                || n.contains("not fetched yet")
                || n.contains("refused catalog read")
        })
        .cloned()
        .collect()
}

pub fn is_legacy(error: &CoreError) -> bool {
    matches!(error, CoreError::LegacyManifest { .. })
}
