//! Authoring validation over a catalog directory: what a maintainer can
//! know about their own content before anyone installs it.
//!
//! Two passes over every item. The structural pass asks whether each
//! harness's loader could hold this item at all — a name it will not
//! accept, a SKILL.md that disagrees with its own directory. The safety
//! pass runs the same rules an install runs, against the same content, so
//! a catalog finds out in its own CI rather than in somebody else's plan
//! preview.
//!
//! Both passes only report what an author can act on. Anything rendering
//! resolves on its own is not a problem this can help with, and naming it
//! would send people to fix something that is not broken.
//!
//! Exit code is the whole point: 1 when something is broken or blocked,
//! and with `--strict`, 1 for advisories and warnings too.

use std::path::{Path, PathBuf};

use kendex_core::model::{HarnessId, ItemKind};
use kendex_core::quality::{self, AuditInput, Content, TreeFile, Verdict};
use kendex_core::render::validate;
use kendex_core::source_read::SealedSource;

use super::{CliResult, say};

/// What one item's two passes produced.
#[derive(Default)]
struct Tally {
    breakage: usize,
    advisory: usize,
    blocked: usize,
    warned: usize,
    items: usize,
}

pub fn run(catalog: &Path, strict: bool) -> CliResult {
    let sealed = SealedSource::open(catalog)?;
    let mut tally = Tally::default();
    for (kind, dir) in [
        (ItemKind::Agent, "agents"),
        (ItemKind::Skill, "skills"),
        (ItemKind::Hook, "hooks"),
        (ItemKind::Command, "commands"),
        (ItemKind::McpServer, "mcp"),
    ] {
        check_dir(&sealed, kind, dir, &mut tally)?;
    }
    say(&format!(
        "{} item(s): {} breakage, {} advisory, {} held back, {} warned",
        tally.items, tally.breakage, tally.advisory, tally.blocked, tally.warned
    ));
    let failing = tally.breakage
        + tally.blocked
        + match strict {
            true => tally.advisory + tally.warned,
            false => 0,
        };
    match failing {
        0 => Ok(()),
        count => {
            Err(format!("{count} problem(s) must be fixed before this catalog installs").into())
        }
    }
}

fn check_dir(sealed: &SealedSource, kind: ItemKind, dir: &str, tally: &mut Tally) -> CliResult {
    let root = sealed.root().join(dir);
    if !sealed.is_dir(&root) {
        return Ok(());
    }
    for entry in sealed.list_dir(&root)? {
        let Some(name) = item_name(&entry) else {
            continue;
        };
        let content = match kind {
            ItemKind::Skill => tree_content(sealed, &entry)?,
            _ if sealed.is_file(&entry) => Content::Document {
                text: sealed.read_to_string(&entry)?,
            },
            _ => continue,
        };
        tally.items += 1;
        let location = entry
            .strip_prefix(sealed.root())
            .unwrap_or(&entry)
            .display()
            .to_string();
        structural(kind, &name, &location, &content, tally);
        safety(kind, &name, &location, content, tally);
    }
    Ok(())
}

/// A skill's whole tree; anything else is one file.
fn tree_content(
    sealed: &SealedSource,
    dir: &Path,
) -> Result<Content, kendex_core::error::CoreError> {
    if !sealed.is_dir(dir) {
        return Ok(Content::Unread {
            why: "a skill is a directory holding SKILL.md",
        });
    }
    Ok(Content::SkillTree {
        files: sealed
            .collect_tree(dir, &[])?
            .into_iter()
            .map(|(path, bytes)| TreeFile::read(path, &bytes))
            .collect(),
    })
}

/// The name the item installs under: a directory's own name, or a file's
/// stem. Dotfiles and anything without a name are not items.
fn item_name(path: &Path) -> Option<String> {
    let raw = match path.is_dir() {
        true => path.file_name(),
        false => path.file_stem(),
    }?;
    let name = raw.to_str()?;
    (!name.starts_with('.')).then(|| name.to_owned())
}

/// Would each harness's loader accept this?
///
/// Only what the author controls. Names are checked against every harness,
/// because a name is carried through untouched. A skill tree is checked
/// once for the things its SKILL.md must say — that it exists, that it
/// names the directory it sits in, that it has a description — and it is
/// deliberately *not* checked against the tightest body cap, because
/// rendering splits an oversized skill into `references/` before it
/// reaches the tool that has that cap. Reporting it here would name a
/// problem the renderer has already solved and send an author off to fix
/// something that is not broken.
fn structural(kind: ItemKind, name: &str, location: &str, content: &Content, tally: &mut Tally) {
    for harness in HarnessId::ALL {
        if !kendex_core::harness::capabilities(harness, kind)
            .install
            .global
        {
            continue;
        }
        let mut findings = validate::validate_name(harness, name);
        if let (Content::SkillTree { files }, HarnessId::Claude) = (content, harness) {
            let files: Vec<(PathBuf, Vec<u8>)> = files
                .iter()
                .map(|file| {
                    let bytes = file.text.clone().unwrap_or_default().into_bytes();
                    (file.path.clone(), bytes)
                })
                .collect();
            // Claude has no body cap, so this pass is the tree's own shape
            // and nothing about any one tool's limits.
            findings.extend(validate::validate_skill_tree(harness, name, name, &files));
        }
        for finding in findings {
            match finding.is_breakage() {
                true => tally.breakage += 1,
                false => tally.advisory += 1,
            }
            let level = match finding.is_breakage() {
                true => "error",
                false => "warning",
            };
            say(&format!(
                "[{level}] {}: {location}: {}",
                harness.name(),
                finding.message
            ));
            say(&format!("    fix: {}", finding.remediation));
        }
    }
}

fn safety(kind: ItemKind, name: &str, location: &str, content: Content, tally: &mut Tally) {
    let result = quality::audit(AuditInput {
        kind,
        name: name.to_owned(),
        harness: None,
        location: location.to_owned(),
        content,
    });
    let thresholds = quality::Thresholds::default();
    let (verdict, _) = quality::verdict(&result.findings, &result.safety, thresholds);
    match verdict {
        Verdict::Block => tally.blocked += 1,
        Verdict::Warn => tally.warned += 1,
        Verdict::Clean => {}
    }
    // Findings carry their own severity rather than being relabelled
    // error/warning: only the verdict below decides whether this run
    // fails, and a line that says "error" without failing anything is a
    // line people learn to scroll past.
    for finding in &result.findings {
        say(&format!(
            "[{}] safety: {}: {} ({})",
            finding.severity.name(),
            finding.location,
            finding.message,
            finding.rule
        ));
        say(&format!("    fix: {}", finding.remediation));
    }
    if verdict != Verdict::Clean {
        say(&format!(
            "[{}] safety: {location}: {} {name} scores {}/100",
            match verdict {
                Verdict::Block => "error",
                _ => "warning",
            },
            kind.name(),
            result.safety.score
        ));
    }
}
