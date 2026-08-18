//! Baseline TSVs — the tighten-only ratchets' frozen rows: `path<TAB>count`,
//! byte-sorted, unique, positive counts. Read through the index like every
//! other policy file; `--update` edits the working-tree copy the user will
//! commit next.

use std::collections::BTreeMap;

use crate::error::Result;

use super::ctx::GuardCtx;
use super::guard_err;
use super::settings::policy_content;

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

/// A counted path the baseline TSV cannot carry — a tab or newline in
/// the name — is a loud refusal, never a garbage row. Checked after
/// excludes apply, so the refusal's own remedy (exclude it) works.
pub fn assert_baseline_representable(check: &str, counts: &BTreeMap<String, u64>) -> Result<()> {
    match counts
        .keys()
        .find(|path| path.contains('\t') || path.contains('\n'))
    {
        None => Ok(()),
        Some(path) => Err(guard_err(
            check,
            format!(
                "tracked path contains a tab or newline, unrepresentable in the baseline TSV (a staged excludes row skips it): {path:?}"
            ),
        )),
    }
}

/// The baseline as the working tree has it — what `--update` edits, since
/// it is editing the file the user will commit next.
pub fn load_worktree_baseline(
    ctx: &GuardCtx,
    check: &str,
    file: &str,
) -> Result<BTreeMap<String, u64>> {
    match crate::fs::read_if_exists(&ctx.root.join(file))? {
        Some(text) => parse_baseline(check, file, &text),
        None => Ok(BTreeMap::new()),
    }
}

/// `--update`'s one rule, shared by every ratchet: rows lower or vanish,
/// never appear or rise. Growth stays a visible hand-edit. `must_go` names
/// the reason a still-counted row no longer belongs (a file back at or
/// under its threshold), `None` to keep it.
pub fn tighten(
    baseline: &BTreeMap<String, u64>,
    counts: &BTreeMap<String, u64>,
    must_go: impl Fn(&str, u64) -> Option<String>,
    out: &mut super::Outcome,
) -> BTreeMap<String, u64> {
    let mut updated = BTreeMap::new();
    for (path, frozen) in baseline {
        let Some(count) = counts.get(path) else {
            out.say(format!("removed: {path} (row {frozen})"));
            continue;
        };
        if let Some(reason) = must_go(path, *count) {
            out.say(format!("removed: {path} ({reason})"));
            continue;
        }
        if count < frozen {
            out.say(format!("tightened: {path} {frozen} -> {count}"));
            updated.insert(path.clone(), *count);
            continue;
        }
        if count > frozen {
            out.say(format!(
                "kept (grew {count} > {frozen} — growth is a hand-edit, never --update): {path}"
            ));
        }
        updated.insert(path.clone(), *frozen);
    }
    updated
}

pub fn parse_baseline(check: &str, file: &str, text: &str) -> Result<BTreeMap<String, u64>> {
    let mut rows = BTreeMap::new();
    let mut previous: Option<String> = None;
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let malformed = || {
            guard_err(
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
            return Err(guard_err(
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
