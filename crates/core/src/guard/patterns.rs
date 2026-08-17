//! Exclusion lists, path classes, and baseline TSVs — the policy files the
//! guards read, all index-aware through [`super::settings::policy_content`].
//!
//! Two pattern dialects, never guessed between. New rules use
//! gitignore-style matching (`*` and `?` stop at `/`, `**` crosses).
//! Patterns imported from v1 keep the legacy shell-glob dialect (`*`
//! crosses `/`, `?` any one character, `[…]` classes) and are marked as
//! imported where they live — a "same file, new matcher" conversion would
//! silently change what is excluded.

use std::collections::BTreeMap;

use crate::error::{CoreError, Result};

use super::ctx::GuardCtx;
use super::settings::policy_content;

fn err(check: &str, message: impl Into<String>) -> CoreError {
    CoreError::Guard {
        check: check.to_owned(),
        message: message.into(),
    }
}

/// The marker line the importer writes at the top of a v1 excludes file.
pub const LEGACY_DIALECT_MARKER: &str = "# vstack-guard-dialect: legacy-glob";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Gitignore,
    LegacyGlob,
}

/// A pattern's shape-check for the small legacy dialect: `*`, `?`, `[…]`
/// classes, everything else literal. Anything the dialect does not define
/// is a refusal, never a guess.
fn validate_pattern(check: &str, pattern: &str) -> Result<()> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '[' {
            let mut end = index + 1;
            if end < chars.len() && (chars[end] == '!' || chars[end] == '^') {
                end += 1;
            }
            if end < chars.len() && chars[end] == ']' {
                end += 1;
            }
            while end < chars.len() && chars[end] != ']' {
                end += 1;
            }
            if end >= chars.len() {
                return Err(err(
                    check,
                    format!("pattern '{pattern}' has an unclosed character class"),
                ));
            }
            index = end;
        }
        index += 1;
    }
    Ok(())
}

fn class_matches(class: &[char], candidate: char) -> bool {
    let (negated, body) = match class.first() {
        Some('!') | Some('^') => (true, &class[1..]),
        _ => (false, class),
    };
    let mut matched = false;
    let mut index = 0;
    while index < body.len() {
        if index + 2 < body.len() && body[index + 1] == '-' {
            if body[index] <= candidate && candidate <= body[index + 2] {
                matched = true;
            }
            index += 3;
        } else {
            if body[index] == candidate {
                matched = true;
            }
            index += 1;
        }
    }
    matched != negated
}

/// One matcher for both dialects, differing only in what a star crosses.
fn glob(pattern: &[char], path: &[char], dialect: Dialect) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    match pattern[0] {
        '*' => {
            // `**` in gitignore crosses separators; a single `*` does not.
            // Legacy `*` always crosses.
            let (crosses, rest) = match (dialect, pattern.get(1)) {
                (Dialect::Gitignore, Some('*')) => (true, &pattern[2..]),
                (Dialect::Gitignore, _) => (false, &pattern[1..]),
                (Dialect::LegacyGlob, _) => (true, &pattern[1..]),
            };
            let mut index = 0;
            loop {
                if glob(rest, &path[index..], dialect) {
                    return true;
                }
                if index >= path.len() {
                    return false;
                }
                if !crosses && path[index] == '/' {
                    // A gitignore `**` may also swallow the separator when
                    // written as a path segment; the plain star stops here.
                    return glob(rest, &path[index..], dialect);
                }
                index += 1;
            }
        }
        '?' => {
            if path.is_empty() {
                return false;
            }
            if dialect == Dialect::Gitignore && path[0] == '/' {
                return false;
            }
            glob(&pattern[1..], &path[1..], dialect)
        }
        '[' => {
            let close = pattern[1..]
                .iter()
                .position(|c| *c == ']')
                .map(|offset| offset + 1);
            // An unclosed class was refused at load; treat defensively as
            // a literal '[' if it ever gets here.
            let Some(close) = close.filter(|close| *close > 1) else {
                return !path.is_empty()
                    && path[0] == '['
                    && glob(&pattern[1..], &path[1..], dialect);
            };
            if path.is_empty() || !class_matches(&pattern[1..close], path[0]) {
                return false;
            }
            glob(&pattern[close + 1..], &path[1..], dialect)
        }
        literal => {
            !path.is_empty() && path[0] == literal && glob(&pattern[1..], &path[1..], dialect)
        }
    }
}

pub fn matches(pattern: &str, path: &str, dialect: Dialect) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let path: Vec<char> = path.chars().collect();
    glob(&pattern, &path, dialect)
}

/// A loaded exclusion list: `pattern<TAB>reason` per line, reason
/// mandatory, `#` comments and blanks ignored, dialect decided by the
/// marker line the importer writes. A missing file is an empty list.
#[derive(Debug)]
pub struct Excludes {
    pub file: String,
    pub dialect: Dialect,
    patterns: Vec<String>,
}

impl Excludes {
    pub fn is_excluded(&self, path: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| matches(pattern, path, self.dialect))
    }
}

pub fn load_excludes(ctx: &GuardCtx, check: &str, file: &str) -> Result<Excludes> {
    let mut excludes = Excludes {
        file: file.to_owned(),
        dialect: Dialect::Gitignore,
        patterns: Vec::new(),
    };
    let Some(bytes) = policy_content(ctx, check, file)? else {
        return Ok(excludes);
    };
    let text = String::from_utf8_lossy(&bytes);
    if text.lines().next().map(str::trim) == Some(LEGACY_DIALECT_MARKER) {
        excludes.dialect = Dialect::LegacyGlob;
    }
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((pattern, reason)) = line.split_once('\t') else {
            return Err(err(
                check,
                format!(
                    "{file}:{}: expected 'pattern<TAB>reason' (every exclusion carries its justification)",
                    number + 1
                ),
            ));
        };
        if pattern.is_empty() || reason.trim().is_empty() {
            return Err(err(
                check,
                format!(
                    "{file}:{}: expected 'pattern<TAB>reason' (every exclusion carries its justification)",
                    number + 1
                ),
            ));
        }
        validate_pattern(check, pattern)?;
        excludes.patterns.push(pattern.to_owned());
    }
    Ok(excludes)
}

/// A tighten-only baseline: `path<TAB>count` rows, byte-sorted, unique,
/// positive counts — hygiene enforced, never repaired. Read through the
/// index like every other policy file. A missing file is an empty
/// baseline.
pub fn load_baseline(ctx: &GuardCtx, check: &str, file: &str) -> Result<BTreeMap<String, u64>> {
    let Some(bytes) = policy_content(ctx, check, file)? else {
        return Ok(BTreeMap::new());
    };
    parse_baseline(check, file, &String::from_utf8_lossy(&bytes))
}

pub fn parse_baseline(check: &str, file: &str, text: &str) -> Result<BTreeMap<String, u64>> {
    let mut rows = BTreeMap::new();
    let mut previous: Option<String> = None;
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let malformed = || {
            err(
                check,
                format!(
                    "{file}:{}: malformed row (expected 'path<TAB>count' with a positive count)",
                    number + 1
                ),
            )
        };
        let (path, count) = line.split_once('\t').ok_or_else(malformed)?;
        let count: u64 = count.parse().map_err(|_| malformed())?;
        if count == 0 || path.is_empty() {
            return Err(malformed());
        }
        if let Some(previous) = &previous
            && path <= previous.as_str()
        {
            return Err(err(
                check,
                format!(
                    "{file}: rows must be byte-sorted and unique ('{path}' after '{previous}')"
                ),
            ));
        }
        previous = Some(path.to_owned());
        rows.insert(path.to_owned(), count);
    }
    Ok(rows)
}

/// A baseline serialized back out, in the shape `load_baseline` demands.
pub fn render_baseline(rows: &BTreeMap<String, u64>) -> String {
    let mut out = String::new();
    for (path, count) in rows {
        out.push_str(path);
        out.push('\t');
        out.push_str(&count.to_string());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_star_crosses_slashes_and_gitignore_star_does_not() {
        assert!(matches("target/*", "target/a/b/c.rs", Dialect::LegacyGlob));
        assert!(!matches("target/*", "target/a/b/c.rs", Dialect::Gitignore));
        assert!(matches("target/*", "target/c.rs", Dialect::Gitignore));
        assert!(matches("target/**", "target/a/b/c.rs", Dialect::Gitignore));
        assert!(matches("**/*.min.js", "a/b/x.min.js", Dialect::Gitignore));
        assert!(matches("*.lock", "Cargo.lock", Dialect::Gitignore));
        assert!(!matches("*.lock", "sub/Cargo.lock", Dialect::Gitignore));
        assert!(matches("*.lock", "sub/Cargo.lock", Dialect::LegacyGlob));
        assert!(matches("a?c", "abc", Dialect::Gitignore));
        assert!(!matches("a?c", "a/c", Dialect::Gitignore));
        assert!(matches("v[12]/x", "v1/x", Dialect::Gitignore));
        assert!(!matches("v[!12]/x", "v1/x", Dialect::Gitignore));
    }

    #[test]
    fn unclosed_class_is_a_refusal_at_load() {
        assert!(validate_pattern("t", "a[bc").is_err());
        assert!(validate_pattern("t", "a[bc]").is_ok());
        assert!(validate_pattern("t", "a[]]").is_ok());
    }

    #[test]
    fn baseline_hygiene_is_enforced_not_repaired() {
        assert!(parse_baseline("t", "b.tsv", "a\t3\nb\t1\n").is_ok());
        assert!(
            parse_baseline("t", "b.tsv", "b\t3\na\t1\n").is_err(),
            "unsorted"
        );
        assert!(
            parse_baseline("t", "b.tsv", "a\t3\na\t1\n").is_err(),
            "duplicate"
        );
        assert!(
            parse_baseline("t", "b.tsv", "a\t0\n").is_err(),
            "zero count"
        );
        assert!(parse_baseline("t", "b.tsv", "a 3\n").is_err(), "no tab");
    }
}
