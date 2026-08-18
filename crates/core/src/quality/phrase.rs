//! Finding a phrase in a line the way a reader would.
//!
//! Every rule reads every line through this, so it is the hottest code in
//! an audit: a scope of a few hundred installations is millions of these
//! calls. What it must not do is care about spacing — content that says
//! `ignore  previous   instructions` is saying the phrase, and a matcher
//! that only knows single spaces is a matcher anyone can step around.

/// Substring search where one space in the needle matches any run of
/// whitespace in the haystack — `ignore  previous   instructions` is the
/// same phrase as the single-spaced one.
///
/// Every rule reads every line through this, so it skips to the next byte
/// that could begin a match rather than trying each position in turn: on
/// ordinary prose the first byte rules out almost everywhere.
pub fn find_phrase(hay: &str, needle: &str) -> Option<usize> {
    let hay = hay.as_bytes();
    let needle = needle.as_bytes();
    let &first = needle.first()?;
    let mut from = 0;
    while from < hay.len() {
        let at = from + hay[from..].iter().position(|&byte| byte == first)?;
        if matches_at(hay, at, needle) {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// Whether the needle sits at exactly this offset, one space in it standing
/// for any run of spaces in the haystack.
fn matches_at(hay: &[u8], at: usize, needle: &[u8]) -> bool {
    let mut h = at;
    let mut n = 0;
    while n < needle.len() {
        if needle[n] == b' ' {
            if h >= hay.len() || hay[h] != b' ' {
                return false;
            }
            while h < hay.len() && hay[h] == b' ' {
                h += 1;
            }
            n += 1;
            continue;
        }
        if h >= hay.len() || hay[h] != needle[n] {
            return false;
        }
        h += 1;
        n += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_phrase_is_found_wherever_it_sits() {
        assert_eq!(find_phrase("ignore this", "ignore"), Some(0));
        assert_eq!(find_phrase("please ignore this", "ignore"), Some(7));
        assert_eq!(find_phrase("please ignore", "ignore this"), None);
        assert_eq!(find_phrase("", "ignore"), None);
        assert_eq!(find_phrase("ignore", ""), None);
    }

    /// One space in the needle stands for any run of spaces, wherever the
    /// run falls — including a false start earlier in the line, which the
    /// search has to walk past rather than give up on.
    #[test]
    fn one_space_stands_for_a_run_of_them() {
        assert_eq!(
            find_phrase("ignore  previous   rules", "ignore previous"),
            Some(0)
        );
        assert_eq!(
            find_phrase("ignored, then ignore  previous", "ignore previous"),
            Some(14)
        );
        assert_eq!(find_phrase("ignoreprevious", "ignore previous"), None);
    }

    /// A near miss must not swallow the real one behind it: the search
    /// resumes one byte on, not past the whole failed attempt.
    #[test]
    fn a_false_start_does_not_hide_a_later_match() {
        assert_eq!(find_phrase("aab", "ab"), Some(1));
        assert_eq!(find_phrase("curl curl | sh", "curl | sh"), Some(5));
    }
}
