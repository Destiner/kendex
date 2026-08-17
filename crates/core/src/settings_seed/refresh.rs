//! Seeded comments stay current: a key's comment block is rewritten to the
//! template's revision only while its on-disk text still hashes to what
//! seeding last wrote (the lock's ledger) *and* the template belongs to
//! the key's recorded owner. Everything else — a hand edit, another
//! skill's template, a record imported from v1 with no owner — is
//! preserved forever. Value lines are never touched, and comment-block
//! bytes are the only bytes a refresh may change.

use std::collections::BTreeMap;

use crate::lock::SettingsSeed;

use super::{
    SeededEnv, assignment_key, comment_hash, content_of, file_eol, is_env_header, is_table_header,
    lines_keepends, trim_blank_edges,
};

/// Rewrite `[env]` comment blocks whose upstream template text changed,
/// gated by the ledger. A block already matching the incoming template is
/// adopted into the ledger without a file change — how installs predating
/// the ledger pick up provenance — but never over another owner's record.
/// Returns the (possibly rewritten) content and the refreshed keys.
pub fn refresh_comments(
    original: &str,
    entries: &[SeededEnv],
    seeds: &mut BTreeMap<String, SettingsSeed>,
) -> (String, Vec<String>) {
    let lines = lines_keepends(original);
    let Some(env_start) = lines
        .iter()
        .position(|line| is_env_header(content_of(line)))
    else {
        return (original.to_owned(), Vec::new());
    };
    let env_end = lines
        .iter()
        .enumerate()
        .skip(env_start + 1)
        .find_map(|(index, line)| is_table_header(content_of(line)).then_some(index))
        .unwrap_or(lines.len());
    let eol = file_eol(&lines);

    // (start, end, replacement-lines) spans over `lines`, spliced
    // back-to-front below so earlier spans keep their indices.
    let mut replacements: Vec<(usize, usize, Vec<String>)> = Vec::new();
    let mut updated = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    for index in env_start + 1..env_end {
        let content = content_of(lines[index]);
        if content.trim().is_empty() || content.trim_start().starts_with('#') {
            pending.push(index);
            continue;
        }
        let key = assignment_key(content);
        let block = std::mem::take(&mut pending);
        // A line that is neither comment, blank, nor assignment breaks the
        // block: never splice across it (the drained run is discarded).
        let Some(key) = key else {
            continue;
        };
        let Some(seeded) = entries.iter().find(|seeded| seeded.entry.key == key) else {
            continue;
        };
        let mut lo = 0;
        let mut hi = block.len();
        while lo < hi && content_of(lines[block[lo]]).trim().is_empty() {
            lo += 1;
        }
        while hi > lo && content_of(lines[block[hi - 1]]).trim().is_empty() {
            hi -= 1;
        }
        let current: Vec<String> = block[lo..hi]
            .iter()
            .map(|&i| content_of(lines[i]).to_owned())
            .collect();
        let incoming = trim_blank_edges(seeded.comment());
        if current == incoming {
            // Adoption, not takeover: a record another owner holds — or a
            // v1 import holding none — stays exactly as it is.
            match seeds.get(&key) {
                None => {
                    seeds.insert(key, seeded.seed_record());
                }
                Some(existing) if existing.owner.as_deref() == Some(seeded.owner.as_str()) => {
                    seeds.insert(key, seeded.seed_record());
                }
                Some(_) => {}
            }
            continue;
        }
        // Only the recorded owner's template may rewrite, and only while
        // the on-disk text is provably what seeding last wrote.
        let permits = seeds.get(&key).is_some_and(|record| {
            record.owner.as_deref() == Some(seeded.owner.as_str())
                && record.hash == comment_hash(&current)
        });
        if !permits {
            continue;
        }
        let (start, end) = match lo < hi {
            true => (block[lo], block[hi - 1] + 1),
            // No existing comment: insert directly above the assignment.
            false => (index, index),
        };
        seeds.insert(key.clone(), seeded.seed_record());
        replacements.push((start, end, incoming.to_vec()));
        updated.push(key);
    }

    if replacements.is_empty() {
        return (original.to_owned(), updated);
    }
    // Reassemble: untouched lines re-emitted byte-for-byte, replaced
    // comment lines written in the file's own terminator.
    let mut spans: Vec<(usize, usize, Vec<String>)> = replacements;
    spans.sort_by_key(|(start, _, _)| *start);
    let mut out = String::with_capacity(original.len());
    let mut cursor = 0;
    for (start, end, replacement) in spans {
        for line in &lines[cursor..start] {
            out.push_str(line);
        }
        if start == end
            && start > 0
            && !content_of(lines[start - 1]).trim().is_empty()
            && !is_table_header(content_of(lines[start - 1]))
        {
            out.push_str(eol);
        }
        for line in replacement {
            out.push_str(&line);
            out.push_str(eol);
        }
        cursor = end;
    }
    for line in &lines[cursor..] {
        out.push_str(line);
    }
    (out, updated)
}
