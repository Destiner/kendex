//! Exclusion lists and path classes — pattern policy the guards read,
//! index-aware through [`super::settings::policy_content`].
//!
//! Two pattern dialects, never guessed between. New rules use
//! gitignore-style matching (`*` and `?` stop at `/`, `**` crosses).
//! Patterns imported from v1 keep the legacy shell-glob dialect (`*`
//! crosses `/`, `?` any one character, `[…]` classes) and are marked as
//! imported where they live — a "same file, new matcher" conversion would
//! silently change what is excluded.

use crate::error::Result;

use super::ctx::GuardCtx;
use super::guard_err;
use super::settings::policy_content;

/// The marker line the importer writes at the top of a v1 excludes file.
pub const LEGACY_DIALECT_MARKER: &str = "# vstack-guard-dialect: legacy-glob";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Gitignore,
    LegacyGlob,
}

/// Where the character class opening at `open` closes: the index of its
/// `]`, past an optional negation and past a `]` in first position, which
/// sh reads as a member rather than the close. `None` = unclosed. The
/// validator and the matcher both read classes through this, so a pattern
/// accepted at load can never be parsed differently at match time.
fn class_end(chars: &[char], open: usize) -> Option<usize> {
    let mut end = open + 1;
    if matches!(chars.get(end), Some('!') | Some('^')) {
        end += 1;
    }
    if chars.get(end) == Some(&']') {
        end += 1;
    }
    (end..chars.len()).find(|&index| chars[index] == ']')
}

/// A pattern's shape-check for the small legacy dialect: `*`, `?`, `[…]`
/// classes, everything else literal. Anything the dialect does not define
/// is a refusal, never a guess.
fn validate_pattern(check: &str, pattern: &str) -> Result<()> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '[' {
            let Some(end) = class_end(&chars, index) else {
                return Err(guard_err(
                    check,
                    format!("pattern '{pattern}' has an unclosed character class"),
                ));
            };
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
            // An unclosed class was refused at load; a literal '[' is the
            // only reading left if one ever gets here.
            let Some(close) = class_end(pattern, 0) else {
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
            return Err(guard_err(
                check,
                format!(
                    "{file}:{}: expected 'pattern<TAB>reason' (every exclusion carries its justification)",
                    number + 1
                ),
            ));
        };
        if pattern.is_empty() || reason.trim().is_empty() {
            return Err(guard_err(
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
        assert!(validate_pattern("t", "a[!]]").is_ok());
        assert!(
            validate_pattern("t", "a[]").is_err(),
            "the ] is a member, not the close"
        );
    }

    /// sh reads a `]` in first position as a member of the class, and the
    /// validator accepts it as one — so the matcher must read it the same
    /// way, or a pattern accepted at load matches the wrong strings.
    #[test]
    fn a_leading_close_bracket_is_a_class_member() {
        assert!(matches("a[]]", "a]", Dialect::LegacyGlob));
        assert!(!matches("a[]]", "a[", Dialect::LegacyGlob));
        assert!(!matches("a[]]", "a[]]", Dialect::LegacyGlob));
        assert!(matches("[!]]x", "ax", Dialect::LegacyGlob));
        assert!(!matches("[!]]x", "]x", Dialect::LegacyGlob));
        assert!(matches("a[]b]", "ab", Dialect::LegacyGlob));
        assert!(matches("a[]b]", "a]", Dialect::LegacyGlob));
    }
}
