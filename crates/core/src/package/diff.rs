//! What changed between two versions of one package, file by file, shaped
//! for display: statuses, line counts, and unified hunks. One diff engine
//! covers both cached version trees and the installed files on disk —
//! which live in no repository, so git cannot see them.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use similar::TextDiff;
use specta::Type;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source_read::SealedSource;

/// One side of the comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case", tag = "at", content = "commit")]
pub enum VersionSel {
    /// The package's source subtree at a commit.
    Commit(String),
    /// What is installed on disk right now.
    Installed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum FileStatus {
    Added,
    Removed,
    Modified,
    /// Holds a NUL byte on either side — compared, never rendered as text.
    Binary,
    /// Past the size or line budget — reported, not diffed.
    TooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum LineKind {
    Context,
    Add,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    /// Forward-slash relative path, whatever the platform.
    pub path: String,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
    /// One side was not valid UTF-8 and is shown lossily.
    pub lossy: bool,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PackageDiff {
    pub files: Vec<FileDiff>,
    pub total_additions: u32,
    pub total_deletions: u32,
    /// The comparison hit a budget; what is shown is a prefix, not the whole.
    pub truncated: bool,
}

/// Budgets, checked before the expensive work: a 256 KB file of one-byte
/// lines can cost more to diff than to download.
const MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_FILE_LINES: usize = 10_000;
const MAX_FILES: usize = 400;
const MAX_TOTAL_LINES: usize = 20_000;
const CONTEXT_LINES: usize = 3;

/// Compare two versions of one package. `Installed` reads what apply would
/// compare — the rendered files for skills and agents (the harness names
/// which rendering, agents render per tool) — so the fork question "what
/// did I change" is answered against real bytes, not source that never hit
/// disk in that form.
pub fn package_diff(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    from: &VersionSel,
    to: &VersionSel,
    harness: Option<HarnessId>,
) -> Result<PackageDiff> {
    let from_files = side(env, scope, kind, name, from, harness)?;
    let to_files = side(env, scope, kind, name, to, harness)?;
    Ok(diff_trees(&from_files, &to_files))
}

type Tree = BTreeMap<String, Vec<u8>>;

fn side(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    sel: &VersionSel,
    harness: Option<HarnessId>,
) -> Result<Tree> {
    match sel {
        VersionSel::Commit(commit) => commit_tree(env, scope, kind, name, commit),
        VersionSel::Installed => installed_tree(env, scope, kind, name, harness),
    }
}

/// The package's source subtree at one commit, read through the sealed
/// reader — a historical commit of a catalog is still a catalog, budgets
/// and symlink refusals included.
fn commit_tree(env: &Env, scope: &Scope, kind: ItemKind, name: &str, commit: &str) -> Result<Tree> {
    let manifest = crate::engine::ops::manifest_for_mutation(env, scope)?;
    let Some(decl) = manifest.declared(kind).get(name) else {
        return Err(CoreError::NotDeclared {
            kind,
            name: name.to_owned(),
        });
    };
    let Some(repo) = manifest
        .sources
        .get(&decl.source)
        .and_then(|s| s.repo.clone())
    else {
        return Err(CoreError::ItemRevUnsupported {
            source_name: decl.source.clone(),
        });
    };
    let key = crate::remote::store::repo_key(&crate::remote::clone_url(env, &repo));
    let root = match crate::remote::store::published(env, &key, commit) {
        Some(root) => root,
        None => {
            let mirror = crate::remote::store::mirror_dir(env, &key);
            if !crate::remote::store::has_commit(&mirror, commit) {
                return Err(CoreError::PinUnavailable {
                    repo,
                    pin: commit.to_owned(),
                    reason: "not in the local mirror — refresh the source first".to_owned(),
                });
            }
            let _guard = crate::remote::store::lock_repo(env, &key)?;
            crate::remote::store::publish(env, &key, &mirror, commit)?
        }
    };
    let sealed = SealedSource::open(&root)?;
    let config = crate::source::source_config(&sealed)?;
    let Some(item_path) = crate::source::find_item(&sealed, &config, kind, name) else {
        return Err(CoreError::ItemMissingAtRev {
            name: name.to_owned(),
            repo,
            commit: commit.to_owned(),
        });
    };
    let mut tree = Tree::new();
    if sealed.is_dir(&item_path) {
        for (rel, bytes) in sealed.collect_tree(&item_path, &[])? {
            tree.insert(slashed(&rel), bytes);
        }
    } else {
        let file = item_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.to_owned());
        tree.insert(file, sealed.read(&item_path)?);
    }
    Ok(tree)
}

/// What is installed on disk right now: the canonical tree for a skill,
/// the rendered file for an agent. Ours, so plain reads.
fn installed_tree(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    harness: Option<HarnessId>,
) -> Result<Tree> {
    let harness = harness.unwrap_or(HarnessId::Claude);
    let path = match kind {
        ItemKind::Skill => crate::engine::desired::skill_canonical(env, scope, name),
        ItemKind::Agent => {
            let Some(dir) =
                crate::engine::desired::native_dir(env, scope, harness, ItemKind::Agent)
            else {
                return Err(CoreError::ItemNotFound {
                    kind,
                    name: name.to_owned(),
                    harness,
                });
            };
            dir.join(crate::render::agent::file_name(harness, name))
        }
        other => {
            return Err(CoreError::ItemNotInSource {
                name: name.to_owned(),
                source_name: format!(
                    "diff against the install does not support {} yet",
                    other.name()
                ),
            });
        }
    };
    if path.is_symlink() || !path.exists() {
        return Err(CoreError::ItemNotFound {
            kind,
            name: name.to_owned(),
            harness,
        });
    }
    let mut tree = Tree::new();
    if path.is_dir() {
        for (rel, bytes) in crate::engine::adopt::read_tree(&path)? {
            tree.insert(slashed(&rel), bytes);
        }
    } else {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.to_owned());
        tree.insert(
            file,
            std::fs::read(&path).map_err(|e| CoreError::io(&path, e))?,
        );
    }
    Ok(tree)
}

fn slashed(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn diff_trees(from: &Tree, to: &Tree) -> PackageDiff {
    let mut files = Vec::new();
    let mut total_additions = 0;
    let mut total_deletions = 0;
    let mut total_lines = 0usize;
    let mut truncated = false;
    let paths: Vec<&String> = {
        let mut all: Vec<&String> = from.keys().chain(to.keys()).collect();
        all.sort();
        all.dedup();
        all
    };
    for path in paths {
        if files.len() >= MAX_FILES {
            truncated = true;
            break;
        }
        let old = from.get(path);
        let new = to.get(path);
        if old == new {
            continue;
        }
        let mut file = diff_file(path, old, new, MAX_TOTAL_LINES.saturating_sub(total_lines));
        total_additions += file.additions;
        total_deletions += file.deletions;
        total_lines += file
            .hunks
            .iter()
            .map(|hunk| hunk.lines.len())
            .sum::<usize>();
        if total_lines >= MAX_TOTAL_LINES {
            truncated = true;
            file.hunks.truncate(file.hunks.len());
        }
        files.push(file);
    }
    PackageDiff {
        files,
        total_additions,
        total_deletions,
        truncated,
    }
}

fn diff_file(
    path: &str,
    old: Option<&Vec<u8>>,
    new: Option<&Vec<u8>>,
    line_budget: usize,
) -> FileDiff {
    let status = match (old, new) {
        (None, Some(_)) => FileStatus::Added,
        (Some(_), None) => FileStatus::Removed,
        _ => FileStatus::Modified,
    };
    let empty = Vec::new();
    let old = old.unwrap_or(&empty);
    let new = new.unwrap_or(&empty);
    if old.contains(&0) || new.contains(&0) {
        return FileDiff {
            path: path.to_owned(),
            status: FileStatus::Binary,
            additions: 0,
            deletions: 0,
            lossy: false,
            hunks: Vec::new(),
        };
    }
    let old_text = String::from_utf8_lossy(old);
    let new_text = String::from_utf8_lossy(new);
    let lossy = matches!(old_text, std::borrow::Cow::Owned(_))
        || matches!(new_text, std::borrow::Cow::Owned(_));
    if old.len() > MAX_FILE_BYTES
        || new.len() > MAX_FILE_BYTES
        || old_text.lines().count() > MAX_FILE_LINES
        || new_text.lines().count() > MAX_FILE_LINES
    {
        return FileDiff {
            path: path.to_owned(),
            status: FileStatus::TooLarge,
            additions: 0,
            deletions: 0,
            lossy,
            hunks: Vec::new(),
        };
    }
    let text_diff = TextDiff::from_lines(old_text.as_ref(), new_text.as_ref());
    let mut additions = 0;
    let mut deletions = 0;
    let mut hunks = Vec::new();
    let mut emitted = 0usize;
    for group in text_diff.grouped_ops(CONTEXT_LINES) {
        let (Some(first), Some(last)) = (group.first(), group.last()) else {
            continue;
        };
        let header = format!(
            "@@ -{},{} +{},{} @@",
            first.old_range().start + 1,
            last.old_range().end - first.old_range().start,
            first.new_range().start + 1,
            last.new_range().end - first.new_range().start,
        );
        let mut lines = Vec::new();
        for op in &group {
            for change in text_diff.iter_changes(op) {
                if emitted >= line_budget {
                    break;
                }
                emitted += 1;
                let kind = match change.tag() {
                    similar::ChangeTag::Equal => LineKind::Context,
                    similar::ChangeTag::Insert => LineKind::Add,
                    similar::ChangeTag::Delete => LineKind::Remove,
                };
                match kind {
                    LineKind::Add => additions += 1,
                    LineKind::Remove => deletions += 1,
                    LineKind::Context => {}
                }
                lines.push(Line {
                    kind,
                    text: change.value().trim_end_matches('\n').to_owned(),
                    old_no: change.old_index().map(|i| i as u32 + 1),
                    new_no: change.new_index().map(|i| i as u32 + 1),
                });
            }
        }
        if !lines.is_empty() {
            hunks.push(Hunk { header, lines });
        }
        if emitted >= line_budget {
            break;
        }
    }
    FileDiff {
        path: path.to_owned(),
        status,
        additions,
        deletions,
        lossy,
        hunks,
    }
}
