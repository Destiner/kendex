//! Reading item names out of a declared-layout catalog's fixed directories.
//! A listed name is always one that installs — `find_item` refuses the rest —
//! so a deceptive or otherwise unusable name is never drawn as a dead row.

use crate::model::ItemKind;
use crate::source_read::SealedSource;

/// The fixed directory and extension a declared-layout catalog keeps one of
/// the file-per-item kinds in. Only those kinds have one.
pub(super) fn fixed_kind_dir(kind: ItemKind) -> (&'static str, &'static str) {
    match kind {
        ItemKind::Hook => ("hooks", "sh"),
        ItemKind::Command => ("commands", "md"),
        ItemKind::McpServer => ("mcp", "toml"),
        _ => unreachable!("only file-per-item kinds live in a fixed dir"),
    }
}

/// The skills one explicit catalog dir holds, the flat v1 shape.
pub(super) fn flat_skills(sealed: &SealedSource, dir: &str) -> Vec<String> {
    let Ok(entries) = sealed.list_dir(&sealed.root().join(dir)) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter(|path| sealed.is_file(&path.join("SKILL.md")))
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        // A listed name is one that installs: find_item refuses the rest, so
        // listing them would only draw dead rows and, for a deceptive name,
        // one whose on-screen spelling is not the name that lands on disk.
        .filter(|name| crate::names::item_problem(name).is_none())
        .collect()
}

pub(super) fn agent_stems(sealed: &SealedSource, dir: &str) -> Vec<String> {
    file_stems(sealed, dir, "md")
}

/// The item names one kind dir holds — every file with the kind's
/// extension, by stem.
fn file_stems(sealed: &SealedSource, dir: &str, ext: &str) -> Vec<String> {
    let Ok(entries) = sealed.list_dir(&sealed.root().join(dir)) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter(|path| path.extension().is_some_and(|e| e == ext) && sealed.is_file(path))
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
        // A listed name is one that installs: find_item refuses the rest.
        .filter(|name| crate::names::item_problem(name).is_none())
        .collect()
}

/// The item names a fixed kind dir holds, by file stem.
pub(super) fn ext_stems(sealed: &SealedSource, dir: &str, ext: &str) -> Vec<String> {
    file_stems(sealed, dir, ext)
}
