//! commit-msg — conventional-commit gate over one message. The header
//! (first non-blank, non-comment line) must be `type(scope)!: subject`;
//! git-generated messages (Merge/Revert/Reapply, fixup!/squash!/amend!)
//! pass unchanged. v1's semantics, carried.

use crate::error::Result;

use super::settings::Policy;
use super::{Outcome, guard_err};

const CHECK: &str = "commit-msg";
const DEFAULT_TYPES: [&str; 11] = [
    "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style", "test",
];

/// The header: the first line that is neither blank nor a `#` comment —
/// git strips comment lines before recording the message.
fn header(message: &str) -> Option<&str> {
    message
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| !line.is_empty() && !line.starts_with('#'))
}

fn git_generated(header: &str) -> bool {
    [
        "Merge ", "Revert ", "Reapply ", "fixup! ", "squash! ", "amend! ",
    ]
    .iter()
    .any(|prefix| header.starts_with(prefix))
}

/// `type(scope)!: subject` — scope and `!` optional; the scope accepts
/// uppercase issue keys and issue numbers; the subject must start with a
/// non-space.
fn conventional(header: &str, types: &[String]) -> bool {
    let Some(kind) = types
        .iter()
        .find_map(|kind| header.strip_prefix(kind.as_str()))
    else {
        return false;
    };
    let mut rest = kind;
    if let Some(after) = rest.strip_prefix('(') {
        let Some(close) = after.find(')') else {
            return false;
        };
        let scope = &after[..close];
        if scope.is_empty()
            || !scope.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '#' | ' ' | '_' | '.' | ',' | '/' | '-')
            })
        {
            return false;
        }
        rest = &after[close + 1..];
    }
    rest = rest.strip_prefix('!').unwrap_or(rest);
    let Some(subject) = rest.strip_prefix(": ") else {
        return false;
    };
    subject.starts_with(|c: char| !c.is_whitespace())
}

/// The strip_prefix trick above matches `feature` against type `feat` and
/// leaves "ure…" as the remainder, which then fails shape — but `fix2` vs
/// `fix` would leave "2:…" and also fail; the longest matching type must
/// win so `refactor` is not consumed as `re`+junk. Types are matched
/// longest-first to keep every prefix honest.
fn ordered(types: &[String]) -> Vec<String> {
    let mut sorted = types.to_vec();
    sorted.sort_by_key(|kind| std::cmp::Reverse(kind.len()));
    sorted
}

pub fn run(policy: &Policy, message: &str) -> Result<Outcome> {
    let types = policy.string_list(CHECK, "types", &DEFAULT_TYPES)?;
    for kind in &types {
        if kind.is_empty()
            || !kind
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(guard_err(
                CHECK,
                format!("types entry '{kind}' is not a lowercase type name"),
            ));
        }
    }
    if types.is_empty() {
        return Err(guard_err(
            CHECK,
            "types resolved empty — at least one type is required",
        ));
    }
    let mut out = Outcome::default();
    let Some(header) = header(message) else {
        out.say("commit-msg FAIL empty commit message (no non-comment content)");
        out.violations = 1;
        return Ok(out);
    };
    if git_generated(header) {
        out.say(format!(
            "commit-msg: OK — git-generated header accepted: {header}"
        ));
        return Ok(out);
    }
    if conventional(header, &ordered(&types)) {
        out.say(format!("commit-msg: OK — conventional header: {header}"));
        return Ok(out);
    }
    out.say(format!("commit-msg FAIL non-conventional header: {header}"));
    out.say(format!(
        "  expected: type(scope)!: subject — scope and '!' optional; types: {}",
        types.join(" ")
    ));
    out.say("  scope accepts uppercase issue keys and issue numbers, e.g. fix(ABC-123): tighten the gate");
    out.say("  git-generated headers (Merge/Revert/Reapply, fixup!/squash!/amend!) pass unchanged");
    out.violations = 1;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types() -> Vec<String> {
        ordered(
            &DEFAULT_TYPES
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn conventional_headers_pass_and_prefix_confusions_fail() {
        assert!(conventional("feat: add a thing", &types()));
        assert!(conventional("fix(ABC-123): tighten the gate", &types()));
        assert!(conventional("fix(#123)!: case-fold IDs", &types()));
        assert!(conventional("refactor(core/guard): split it", &types()));
        assert!(!conventional("feature: add a thing", &types()));
        assert!(!conventional("feat:missing space", &types()));
        assert!(!conventional("feat: ", &types()));
        assert!(!conventional("feat(): empty scope", &types()));
        assert!(!conventional("unknown: type", &types()));
    }

    #[test]
    fn header_skips_blanks_and_comments_and_git_generated_passes() {
        assert_eq!(header("\n# comment\n\nfeat: x\n"), Some("feat: x"));
        assert!(git_generated("Merge branch 'main'"));
        assert!(git_generated("fixup! feat: x"));
        assert!(!git_generated("feat: merge the lanes"));
    }
}
