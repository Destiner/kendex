//! The safety findings in installed content, the decisions recorded about
//! them, and the verbs that make and take back those decisions.
//!
//! Every finding is printed with the token that names exactly it on exactly
//! this content; `dismiss` takes that token and nothing looser, the way
//! `--allow-unsafe` takes `name@hash` — a bare name in a shell history must
//! never dismiss whatever replaced what was read.

use clap::Args;
use vstack_core::apply;
use vstack_core::engine::decisions::{DecisionState, short_token};
use vstack_core::engine::ops::{
    DecisionRecord, DismissTarget, RecordState, dismiss, list_decisions, revoke_dismissal,
    revoke_override,
};
use vstack_core::engine::{ItemSafety, allow_unsafe_flag, observed_safety};
use vstack_core::env::Env;
use vstack_core::quality::reviews::DismissReason;

use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

#[derive(Args)]
pub struct FindingsArgs {
    #[arg(short = 'g', long)]
    global: bool,
    /// project | global | all (default all)
    #[arg(long)]
    scope: Option<String>,
}

/// What the safety rules found in what is installed right now, each finding
/// with the token a dismissal takes and what has already been decided about
/// it.
pub fn findings(env: &Env, args: FindingsArgs) -> CliResult {
    let filter = ScopeFilter::resolve(args.scope.as_deref(), args.global, ScopeFilter::All)?;
    for scope in resolve_scopes(env, filter)? {
        let rows = observed_safety(env, &scope)?;
        let mut rows: Vec<&ItemSafety> = rows.iter().filter(|r| !r.findings.is_empty()).collect();
        if rows.is_empty() {
            say(&format!("{}: nothing found", scope.label()));
            continue;
        }
        rows.sort_by_key(|row| (!row.blocked(), row.safety.score));
        say(&format!("{}:", scope.label()));
        for row in rows {
            print_row(row);
        }
    }
    Ok(())
}

fn print_row(row: &ItemSafety) {
    let held = match row.blocked() {
        true => " — held back",
        false => "",
    };
    say(&format!(
        "  {} {} for {} scores {}/100{held}",
        row.kind.name(),
        row.name,
        row.harness.display_name(),
        row.safety.score
    ));
    for (finding, decision) in row.findings.iter().zip(&row.decisions) {
        say(&format!(
            "    [{}] {}: {}",
            finding.severity.name(),
            finding.location,
            finding.message
        ));
        say(&format!("      fix: {}", finding.remediation));
        match &decision.state {
            DecisionState::Open { earlier } => {
                if let Some(token) = &decision.token {
                    say(&format!("      token: {}", short_token_of(token)));
                }
                if let Some(earlier) = earlier {
                    say(&format!("      dismissed before, but {earlier}"));
                }
            }
            DecisionState::Dismissed {
                reason,
                dismissed_at,
            } => say(&format!(
                "      dismissed {dismissed_at} — {}",
                reason.name()
            )),
            DecisionState::Accepted { granted_at } => {
                say(&format!("      accepted {granted_at}"));
            }
        }
    }
    if let Some(review_hash) = &row.review_hash
        && row.blocked()
    {
        say(&format!(
            "    to install it anyway, review the findings above and apply with --allow-unsafe {}",
            allow_unsafe_flag(&row.name, review_hash)
        ));
    }
}

/// The token as printed: the backend issues it with the full hash, and a
/// person types back the same prefix the accept flag uses.
fn short_token_of(token: &str) -> String {
    match vstack_core::engine::decisions::DecisionToken::parse(token) {
        Some(parsed) => short_token(&parsed.key, &parsed.fingerprint, &parsed.hash),
        None => token.to_owned(),
    }
}

#[derive(Args)]
pub struct DismissArgs {
    /// The finding tokens printed by `vstack findings`
    #[arg(required = true)]
    tokens: Vec<String>,
    /// wrong-call | intended | trusted-source
    #[arg(long)]
    reason: String,
    #[arg(short = 'g', long)]
    global: bool,
    /// project | global (default project)
    #[arg(long)]
    scope: Option<String>,
}

/// Record that these findings are not problems. One journaled manifest
/// write for the scope; a token that no longer names what is installed
/// stops the whole call before it.
pub fn dismiss_cmd(env: &Env, args: DismissArgs) -> CliResult {
    let reason = DismissReason::parse(&args.reason).ok_or_else(|| {
        format!(
            "unknown --reason '{}'; expected one of: {}",
            args.reason,
            DismissReason::ALL
                .iter()
                .map(|r| r.name())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let targets = args
        .tokens
        .iter()
        .map(|token| DismissTarget::parse(token))
        .collect::<Result<Vec<_>, _>>()?;
    let filter = ScopeFilter::resolve(args.scope.as_deref(), args.global, ScopeFilter::Project)?;
    for scope in resolve_scopes(env, filter)? {
        let plan = dismiss(env, &scope, &targets, reason)?;
        apply::execute(env, &plan, None)?;
        say(&format!(
            "{}: dismissed {} finding{} as {}",
            scope.label(),
            targets.len(),
            if targets.len() == 1 { "" } else { "s" },
            reason.name()
        ));
    }
    Ok(())
}

#[derive(Args)]
pub struct DecisionsArgs {
    /// Take a decision back by its id: kind:name:harness for an acceptance,
    /// kind:name:harness#fingerprint for a dismissal
    #[arg(long)]
    revoke: Vec<String>,
    #[arg(short = 'g', long)]
    global: bool,
    /// project | global | all (default all)
    #[arg(long)]
    scope: Option<String>,
}

/// Every recorded decision — acceptances and dismissals — with whether it
/// still describes what is installed, and the way out of one.
pub fn decisions(env: &Env, args: DecisionsArgs) -> CliResult {
    let default = match args.revoke.is_empty() {
        true => ScopeFilter::All,
        false => ScopeFilter::Project,
    };
    let filter = ScopeFilter::resolve(args.scope.as_deref(), args.global, default)?;
    for scope in resolve_scopes(env, filter)? {
        for id in &args.revoke {
            let plan = match id.split_once('#') {
                Some((key, fingerprint)) => revoke_dismissal(env, &scope, key, fingerprint, None)?,
                None => revoke_override(env, &scope, id)?,
            };
            apply::execute(env, &plan, None)?;
            say(&format!("{}: took back the decision {id}", scope.label()));
        }
        if !args.revoke.is_empty() {
            continue;
        }
        let recorded = list_decisions(env, &scope)?;
        if recorded.is_empty() {
            say(&format!("{}: no decisions recorded", scope.label()));
            continue;
        }
        say(&format!("{}:", scope.label()));
        for decision in recorded {
            let state = match &decision.state {
                RecordState::Active => "active".to_owned(),
                RecordState::Stale { why } => format!("stale: {why}"),
                RecordState::Obsolete => {
                    "obsolete: the item is no longer installed here".to_owned()
                }
            };
            match &decision.record {
                DecisionRecord::Accepted {
                    findings,
                    granted_at,
                } => say(&format!(
                    "  accepted   {} — {findings} finding{} accepted {granted_at} [{state}]",
                    decision.key,
                    if *findings == 1 { "" } else { "s" }
                )),
                DecisionRecord::Dismissed {
                    fingerprint,
                    reason,
                    dismissed_at,
                    finding,
                } => {
                    say(&format!(
                        "  dismissed  {}#{fingerprint} — {} {dismissed_at} [{state}]",
                        decision.key,
                        reason.name()
                    ));
                    if let Some(finding) = finding {
                        say(&format!(
                            "             [{}] {}: {}",
                            finding.severity.name(),
                            finding.location,
                            finding.message
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
