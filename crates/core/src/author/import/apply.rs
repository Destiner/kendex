//! The copy itself: previewed selections into an authored catalog, with
//! every refusal decided before the first byte is written.

use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::model::{ItemKind, Scope};

use super::{Bytes, CandidateGroup, ImportOutcome, ImportSelection, inventory};

/// Apply the wizard's selections to `target`. The inventory is recomputed
/// so every hash is revalidated — bytes that moved since the preview are a
/// refusal, never a stale copy. All refusals are found before anything is
/// written; a refused apply writes nothing.
pub fn apply(
    env: &Env,
    scopes: &[Scope],
    target: &Path,
    selections: &[ImportSelection],
) -> Result<ImportOutcome> {
    let candidates = inventory(env, scopes)?;
    let mut writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let mut outcome = ImportOutcome {
        written: Vec::new(),
        already_present: Vec::new(),
    };
    for selection in selections {
        if let Some(problem) = crate::names::item_problem(&selection.destination) {
            return Err(CoreError::Authoring {
                message: format!(
                    "'{}' cannot name an imported {} — {problem}",
                    selection.destination,
                    selection.kind.name()
                ),
            });
        }
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.kind == selection.kind && candidate.name == selection.name)
            .ok_or_else(|| CoreError::Authoring {
                message: format!(
                    "{} '{}' is no longer on this machine — re-open the import to re-preview",
                    selection.kind.name(),
                    selection.name
                ),
            })?;
        let origin = candidate
            .origins
            .iter()
            .find(|origin| origin.hash == selection.hash && !origin.hash.is_empty())
            .ok_or_else(|| CoreError::Authoring {
                message: format!(
                    "the bytes of {} '{}' changed since the preview — re-open the import to re-preview",
                    selection.kind.name(),
                    selection.name
                ),
            })?;
        license_gate(selection, &origin.group)?;
        let bytes = super::selection_bytes(env, scopes, selection)?;
        collect_writes(target, selection, &bytes, &mut writes, &mut outcome)?;
    }
    for (path, bytes) in &writes {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        std::fs::write(path, bytes).map_err(|e| CoreError::io(path, e))?;
    }
    Ok(outcome)
}

/// Marketplace-origin content copies only past licence evidence: a shown
/// licence the person confirmed, or an explicit basis they stated when no
/// licence was detectable. Confirmation never synthesizes permission.
fn license_gate(selection: &ImportSelection, group: &CandidateGroup) -> Result<()> {
    let CandidateGroup::Marketplace {
        source, license, ..
    } = group
    else {
        return Ok(());
    };
    match license {
        Some(_) if selection.license_confirmed => Ok(()),
        Some(license) => Err(CoreError::Authoring {
            message: format!(
                "'{}' comes from marketplace '{source}' under licence {license} — confirm the licence permits republishing, or pick another origin",
                selection.name
            ),
        }),
        None => match selection.license_basis.as_deref().map(str::trim) {
            Some(basis) if !basis.is_empty() => Ok(()),
            _ => Err(CoreError::Authoring {
                message: format!(
                    "'{}' comes from marketplace '{source}' with no detectable licence — state your basis for copying it (--license-basis), or pick another origin",
                    selection.name
                ),
            }),
        },
    }
}

/// Where each kind lands inside a catalog.
fn destination(target: &Path, kind: ItemKind, name: &str) -> PathBuf {
    match kind {
        ItemKind::Skill => target.join("skills").join(name),
        ItemKind::Agent => target.join("agents").join(format!("{name}.md")),
        ItemKind::Hook => target.join("hooks").join(format!("{name}.sh")),
        ItemKind::Command => target.join("commands").join(format!("{name}.md")),
        ItemKind::McpServer => target.join("mcp").join(format!("{name}.toml")),
        ItemKind::Plugin | ItemKind::PiExtension => target.join(name),
    }
}

fn collect_writes(
    target: &Path,
    selection: &ImportSelection,
    bytes: &Bytes,
    writes: &mut Vec<(PathBuf, Vec<u8>)>,
    outcome: &mut ImportOutcome,
) -> Result<()> {
    if !matches!(
        selection.kind,
        ItemKind::Skill
            | ItemKind::Agent
            | ItemKind::Hook
            | ItemKind::Command
            | ItemKind::McpServer
    ) {
        return Err(CoreError::Authoring {
            message: format!(
                "a {} cannot be imported into a catalog directly",
                selection.kind.name()
            ),
        });
    }
    let dest = destination(target, selection.kind, &selection.destination);
    fold_collision(&dest, &selection.destination)?;
    match bytes {
        Bytes::File(bytes) => {
            if dest.exists() {
                let existing = std::fs::read(&dest).map_err(|e| CoreError::io(&dest, e))?;
                match existing == *bytes {
                    true => outcome.already_present.push(rel_name(target, &dest)),
                    false => {
                        return Err(occupied(&dest, &selection.name));
                    }
                }
                return Ok(());
            }
            outcome.written.push(rel_name(target, &dest));
            writes.push((dest, bytes.clone()));
        }
        Bytes::Tree(files) => {
            if dest.exists() {
                let existing = crate::hash::hash_tree(&dest).unwrap_or_default();
                match existing == crate::hash::hash_files(files) {
                    true => outcome.already_present.push(rel_name(target, &dest)),
                    false => return Err(occupied(&dest, &selection.name)),
                }
                return Ok(());
            }
            outcome.written.push(rel_name(target, &dest));
            for (rel, bytes) in files {
                writes.push((dest.join(rel), bytes.clone()));
            }
        }
    }
    Ok(())
}

/// A sibling whose name folds to the destination's spelling occupies it on
/// a case-insensitive filesystem — refused before the copy, naming both.
fn fold_collision(dest: &Path, name: &str) -> Result<()> {
    let Some(parent) = dest.parent() else {
        return Ok(());
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Ok(());
    };
    let Some(leaf) = dest
        .file_name()
        .map(|leaf| leaf.to_string_lossy().into_owned())
    else {
        return Ok(());
    };
    let folded = crate::names::fold(&leaf);
    for entry in entries.flatten() {
        let sibling = entry.file_name().to_string_lossy().into_owned();
        if sibling != leaf && crate::names::fold(&sibling) == folded {
            return Err(CoreError::Authoring {
                message: format!(
                    "'{name}' would collide with existing '{sibling}' on a case-insensitive filesystem — pick another destination name"
                ),
            });
        }
    }
    Ok(())
}

fn occupied(dest: &Path, name: &str) -> CoreError {
    CoreError::Authoring {
        message: format!(
            "{} already holds different bytes than '{name}' — rename the import destination or remove the existing file first",
            dest.display()
        ),
    }
}

fn rel_name(target: &Path, dest: &Path) -> String {
    dest.strip_prefix(target)
        .unwrap_or(dest)
        .display()
        .to_string()
}
