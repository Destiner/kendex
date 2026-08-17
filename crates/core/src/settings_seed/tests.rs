use std::collections::BTreeMap;

use super::*;

const TEMPLATE: &str = "# ignored preamble\n[env]\n# Which reviewer set to run.\n# Comma separated.\nREVIEWERS = \"arch,security\"\n\nDEPTH = \"2\"\n\n[other]\nX = \"1\"\n";

fn seeded(template: &str, owner: &str) -> Vec<SeededEnv> {
    extract_env_entries(template)
        .into_iter()
        .map(|entry| SeededEnv {
            entry,
            owner: owner.to_owned(),
        })
        .collect()
}

/// The ledger as seeding would have left it for these entries.
fn ledger_for(entries: &[SeededEnv]) -> BTreeMap<String, SettingsSeed> {
    entries
        .iter()
        .map(|seeded| (seeded.entry.key.clone(), seeded.seed_record()))
        .collect()
}

#[test]
fn entries_carry_their_comment_blocks() {
    let entries = extract_env_entries(TEMPLATE);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key, "REVIEWERS");
    assert_eq!(entries[0].lines.len(), 3);
    assert_eq!(entries[1].key, "DEPTH");
}

#[test]
fn merge_is_write_if_absent_with_file_wide_uniqueness() {
    let entries = seeded(TEMPLATE, "review");
    // Key already assigned under a DIFFERENT table still blocks the add.
    let existing = "[custom]\nREVIEWERS = \"mine\"\n\n[env]\nOTHER = \"x\"\n";
    let (merged, added) = merge(Some(existing), &entries).unwrap();
    assert_eq!(added, ["DEPTH"]);
    assert!(merged.contains("REVIEWERS = \"mine\""));
    assert!(!merged.contains("arch,security"));
    assert!(merged.contains("DEPTH = \"2\""));
    assert!(merged.ends_with("DEPTH = \"2\"\n"));

    assert!(merge(Some(&merged), &entries).is_none());
}

#[test]
fn fresh_file_gets_the_seeded_header() {
    let entries = seeded(TEMPLATE, "review");
    let (created, added) = merge(None, &entries).unwrap();
    assert_eq!(added, ["REVIEWERS", "DEPTH"]);
    assert!(created.starts_with("# Public vstack settings"));
    assert!(created.contains("[env]\n# Which reviewer set to run."));
}

#[test]
fn merge_changes_nothing_outside_the_inserted_block() {
    let entries = seeded("[env]\nDEPTH = \"2\"\n", "review");
    let original = "# mine\r\n[env]\r\nREVIEWERS = \"mine\"\r\n\r\n[custom]\r\nX = \"1\"\r\n";
    let (merged, added) = merge(Some(original), &entries).unwrap();
    assert_eq!(added, ["DEPTH"]);
    // The block lands inside [env] in the file's own line terminator; every
    // original byte survives, in order.
    assert!(merged.contains("DEPTH = \"2\"\r\n"), "{merged:?}");
    let without_block = merged.replacen("DEPTH = \"2\"\r\n\r\n", "", 1);
    assert_eq!(without_block, original, "only the block was inserted");
}

#[test]
fn merge_at_file_end_repairs_a_missing_terminator_once() {
    let entries = seeded("[env]\nDEPTH = \"2\"\n", "review");
    let original = "[env]\nREVIEWERS = \"mine\"";
    let (merged, _) = merge(Some(original), &entries).unwrap();
    assert!(
        merged.starts_with("[env]\nREVIEWERS = \"mine\"\n"),
        "{merged:?}"
    );
    assert!(merged.ends_with("DEPTH = \"2\"\n"));
}

#[test]
fn unedited_seeded_comment_refreshes_when_the_template_revises_it() {
    let v1 = seeded("[env]\n# Old words.\nDEPTH = \"2\"\n", "review");
    let mut ledger = ledger_for(&v1);
    let file = "[env]\n# Old words.\nDEPTH = \"9\"\n";

    let v2 = seeded(
        "[env]\n# New words.\n# Two lines now.\nDEPTH = \"2\"\n",
        "review",
    );
    let (out, updated) = refresh_comments(file, &v2, &mut ledger);
    assert_eq!(updated, ["DEPTH"]);
    assert_eq!(
        out,
        "[env]\n# New words.\n# Two lines now.\nDEPTH = \"9\"\n"
    );
    assert_eq!(
        ledger.get("DEPTH").unwrap().hash,
        comment_hash(&["# New words.".to_owned(), "# Two lines now.".to_owned()]),
        "the ledger follows the rewrite"
    );

    // Idempotent: running again changes nothing.
    let (again, updated) = refresh_comments(&out, &v2, &mut ledger);
    assert_eq!(again, out);
    assert!(updated.is_empty());
}

#[test]
fn a_hand_edited_comment_is_preserved_forever() {
    let v1 = seeded("[env]\n# Old words.\nDEPTH = \"2\"\n", "review");
    let mut ledger = ledger_for(&v1);
    let file = "[env]\n# My own explanation.\nDEPTH = \"9\"\n";

    let v2 = seeded("[env]\n# New words.\nDEPTH = \"2\"\n", "review");
    let (out, updated) = refresh_comments(file, &v2, &mut ledger);
    assert_eq!(out, file, "a hand-edited comment never rewrites");
    assert!(updated.is_empty());
}

#[test]
fn value_lines_are_untouched_byte_for_byte() {
    let v1 = seeded("[env]\n# Old.\nDEPTH = \"2\"\n", "review");
    let mut ledger = ledger_for(&v1);
    // Odd spacing and a trailing comment on the value line must survive.
    let file = "[env]\n# Old.\nDEPTH   =\t\"9\"   # keep me\n";
    let v2 = seeded("[env]\n# New.\nDEPTH = \"2\"\n", "review");
    let (out, _) = refresh_comments(file, &v2, &mut ledger);
    assert_eq!(out, "[env]\n# New.\nDEPTH   =\t\"9\"   # keep me\n");
}

#[test]
fn crlf_and_missing_terminator_survive_outside_the_comment_block() {
    let v1 = seeded("[env]\n# Old.\nDEPTH = \"2\"\n", "review");
    let mut ledger = ledger_for(&v1);
    let file = "# head\r\n[env]\r\n# Old.\r\nDEPTH = \"9\"\r\n\r\n[custom]\r\nX = \"1\"";
    let v2 = seeded("[env]\n# New.\nDEPTH = \"2\"\n", "review");
    let (out, updated) = refresh_comments(file, &v2, &mut ledger);
    assert_eq!(updated, ["DEPTH"]);
    assert_eq!(
        out, "# head\r\n[env]\r\n# New.\r\nDEPTH = \"9\"\r\n\r\n[custom]\r\nX = \"1\"",
        "comment bytes are the only bytes that changed"
    );
}

#[test]
fn another_owners_template_never_rewrites_a_seeded_comment() {
    let original_owner = seeded("[env]\n# Old.\nDEPTH = \"2\"\n", "review");
    let mut ledger = ledger_for(&original_owner);
    let file = "[env]\n# Old.\nDEPTH = \"9\"\n";
    // A different skill now ships the same key with new words.
    let intruder = seeded("[env]\n# Their words.\nDEPTH = \"3\"\n", "other-skill");
    let (out, updated) = refresh_comments(file, &intruder, &mut ledger);
    assert_eq!(out, file);
    assert!(updated.is_empty());
    assert_eq!(
        ledger.get("DEPTH").unwrap().owner.as_deref(),
        Some("review"),
        "the record stays with its owner"
    );
}

#[test]
fn a_v1_imported_record_verifies_but_never_rewrites() {
    let mut ledger = BTreeMap::new();
    ledger.insert(
        "DEPTH".to_owned(),
        SettingsSeed {
            owner: None,
            hash: comment_hash(&["# Old.".to_owned()]),
        },
    );
    let file = "[env]\n# Old.\nDEPTH = \"9\"\n";
    let v2 = seeded("[env]\n# New.\nDEPTH = \"2\"\n", "review");
    let (out, updated) = refresh_comments(file, &v2, &mut ledger);
    assert_eq!(out, file, "legacy-owned: preserved, never rewritten");
    assert!(updated.is_empty());
    assert_eq!(ledger.get("DEPTH").unwrap().owner, None);
}

#[test]
fn bootstrap_adopts_a_template_matching_comment_and_freezes_an_edited_one() {
    // Pre-ledger install: the file matches the current template exactly.
    let mut ledger = BTreeMap::new();
    let file = "[env]\n# Current words.\nDEPTH = \"9\"\n";
    let current = seeded("[env]\n# Current words.\nDEPTH = \"2\"\n", "review");
    let (out, updated) = refresh_comments(file, &current, &mut ledger);
    assert_eq!(out, file);
    assert!(updated.is_empty());
    assert_eq!(
        ledger.get("DEPTH").unwrap().owner.as_deref(),
        Some("review"),
        "a matching comment is adopted into the ledger"
    );
    // The next template revision now refreshes it.
    let revised = seeded("[env]\n# Revised words.\nDEPTH = \"2\"\n", "review");
    let (out, updated) = refresh_comments(&out, &revised, &mut ledger);
    assert_eq!(updated, ["DEPTH"]);
    assert!(out.contains("# Revised words."));

    // Pre-ledger install whose comment differs: hand-edited, never adopted.
    let mut ledger = BTreeMap::new();
    let edited = "[env]\n# Somebody's own words.\nDEPTH = \"9\"\n";
    let (out, updated) = refresh_comments(edited, &revised, &mut ledger);
    assert_eq!(out, edited);
    assert!(updated.is_empty());
    assert!(!ledger.contains_key("DEPTH"));
}

#[test]
fn comment_hash_matches_v1s_algorithm() {
    // Locked against v1's `format!("{:016x}", fnv1a(join("\n")))` so
    // imported ledgers verify without re-guessing.
    assert_eq!(comment_hash(&[]), "cbf29ce484222325");
    assert_eq!(comment_hash(&["# a".to_owned()]), {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in b"# a" {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{h:016x}")
    });
}
