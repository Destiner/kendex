//! What a mirror's history says about one directory: the commits that
//! changed it, the tags that name them, and whether it moved between two
//! commits. Everything here reads the bare mirror through [`Hardened`], with
//! full commit ids, a `--` separator, and literal pathspecs — a catalog
//! chooses its own directory names, and a name shaped like git syntax must
//! stay a name.

use std::path::Path;

use crate::process::Hardened;

/// One commit that changed the subtree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRow {
    /// Full commit id.
    pub commit: String,
    /// ISO-8601 committer date.
    pub date: String,
    /// Commit subject, bounded and control-stripped for display.
    pub summary: String,
    /// Tag names pointing at exactly this commit.
    pub tags: Vec<String>,
}

/// The timeline is bounded twice: rows, and the bytes read before parsing.
/// A hostile repository can put megabytes in one subject line; the cap
/// keeps that its problem.
const MAX_ROWS: usize = 200;
const MAX_OUTPUT: usize = 1_000_000;
/// Display bound for one commit subject.
const MAX_SUMMARY: usize = 200;

fn stdout_capped(git: Hardened) -> Option<String> {
    let output = git.run().ok()?;
    if !output.status.success() {
        return None;
    }
    let mut bytes = output.stdout;
    bytes.truncate(MAX_OUTPUT);
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// A subtree path as a pathspec git will not interpret.
fn literal(rel: &Path) -> String {
    format!(":(literal){}", rel.display())
}

/// A commit subject as something safe to show: control characters become
/// spaces (the same posture every displayed foreign string takes) and the
/// length is bounded.
fn shown_summary(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .take(MAX_SUMMARY)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    cleaned.trim().to_owned()
}

/// The commits that changed `rel`, newest first, walking first-parent
/// history from `tip`. Tag names arrive through `%D` decorations so one
/// invocation answers both "what changed" and "what is it called".
pub fn subtree_log(mirror: &Path, tip: &str, rel: &Path) -> Vec<CommitRow> {
    let max = MAX_ROWS.to_string();
    let Some(text) = stdout_capped(Hardened::git_bare(
        mirror,
        &[
            "log",
            "--first-parent",
            "--max-count",
            &max,
            "--decorate=full",
            "--format=%H%x00%cI%x00%s%x00%D%x1e",
            tip,
            "--",
            &literal(rel),
        ],
    )) else {
        return Vec::new();
    };
    text.split('\u{1e}')
        .filter_map(|record| {
            let mut fields = record.trim_start_matches(['\n', ' ']).split('\0');
            let commit = fields.next()?.trim().to_owned();
            if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            let date = fields.next()?.to_owned();
            let summary = shown_summary(fields.next()?);
            let tags = fields
                .next()
                .map(|decorations| {
                    decorations
                        .split(", ")
                        .filter_map(|d| d.trim().strip_prefix("tag: refs/tags/"))
                        .map(shown_summary)
                        .collect()
                })
                .unwrap_or_default();
            Some(CommitRow {
                commit,
                date,
                summary,
                tags,
            })
        })
        .collect()
}

/// Whether `rel` differs between two commits — the signal that an upstream
/// move actually touched this package, rather than the repository around it.
pub fn subtree_changed(mirror: &Path, old: &str, new: &str, rel: &Path) -> bool {
    if old == new {
        return false;
    }
    let output =
        Hardened::git_bare(mirror, &["diff", "--quiet", old, new, "--", &literal(rel)]).run();
    match output {
        // Exit 1 means the paths differ; exit 0 means they do not. Any
        // other outcome means the question could not be answered, and "no
        // update" is the reading that nags nobody about content it cannot
        // show.
        Ok(output) => output.status.code() == Some(1),
        Err(_) => false,
    }
}

/// The newest commit at-or-before `from` that changed `rel` — the content
/// revision an installation at `from` actually holds. An installed commit
/// that merely sat near the package (it changed other files) is not itself
/// on the package's timeline; this maps it onto the row that is.
pub fn last_content_commit(mirror: &Path, from: &str, rel: &Path) -> Option<String> {
    let text = stdout_capped(Hardened::git_bare(
        mirror,
        &[
            "log",
            "--first-parent",
            "--max-count",
            "1",
            "--format=%H",
            from,
            "--",
            &literal(rel),
        ],
    ))?;
    let commit = text.trim().to_owned();
    (commit.len() == 40).then_some(commit)
}
