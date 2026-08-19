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
//! This lives in core because the CLI's `check --catalog`, the indexer's
//! per-package verdicts, and authoring preflight all ask the same two
//! questions of the same bytes — one implementation, one answer.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;
use crate::model::{HarnessId, ItemKind};
use crate::quality::{self, AuditInput, Content, TreeFile, Verdict};
use crate::render::validate;
use crate::source_read::SealedSource;

/// The versioned envelope `check --catalog --json` wraps this report in.
pub const CHECK_SCHEMA: u32 = 1;

/// The `pass` a safety finding carries; structural findings carry the
/// harness whose loader complained.
pub const SAFETY_PASS: &str = "safety";

/// The fixed kind directories the authoring check reads.
const KIND_DIRS: [(ItemKind, &str); 5] = [
    (ItemKind::Agent, "agents"),
    (ItemKind::Skill, "skills"),
    (ItemKind::Hook, "hooks"),
    (ItemKind::Command, "commands"),
    (ItemKind::McpServer, "mcp"),
];

/// One problem either pass found, carrying everything a machine consumer
/// needs to place it. Field order is the JSON field order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckFinding {
    /// The file within the catalog — for safety findings, the rule's own
    /// location, which may name a file inside a skill tree.
    pub file: String,
    pub kind: &'static str,
    pub name: String,
    /// The harness whose loader complains, or [`SAFETY_PASS`].
    pub pass: String,
    /// `error`/`warning` for structural findings; the safety severity
    /// (`low`..`critical`) for safety findings.
    pub severity: &'static str,
    /// The safety rule that fired; `None` for structural findings.
    pub rule: Option<String>,
    pub message: String,
    pub fix: String,
}

impl CheckFinding {
    pub fn is_breakage(&self) -> bool {
        self.rule.is_none() && self.severity == "error"
    }
}

/// One item with both passes run over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedItem {
    pub kind: ItemKind,
    pub name: String,
    /// The item's own path within the catalog.
    pub file: String,
    /// Structural findings first, then safety, in report order.
    pub findings: Vec<CheckFinding>,
    pub verdict: Verdict,
    pub score: u32,
}

/// What both passes over a whole catalog produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogCheck {
    pub items: Vec<CheckedItem>,
}

/// The counts the summary line and the exit code are made of.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckTally {
    pub items: usize,
    pub breakage: usize,
    pub advisory: usize,
    pub held_back: usize,
    pub warned: usize,
}

impl CatalogCheck {
    pub fn tally(&self) -> CheckTally {
        let mut tally = CheckTally {
            items: self.items.len(),
            ..CheckTally::default()
        };
        for item in &self.items {
            for finding in &item.findings {
                match (finding.rule.is_none(), finding.is_breakage()) {
                    (true, true) => tally.breakage += 1,
                    (true, false) => tally.advisory += 1,
                    _ => {}
                }
            }
            match item.verdict {
                Verdict::Block => tally.held_back += 1,
                Verdict::Warn => tally.warned += 1,
                Verdict::Clean => {}
            }
        }
        tally
    }

    /// How many problems fail the run: breakage and blocked items always,
    /// advisories and warnings only under `strict`.
    pub fn failing(&self, strict: bool) -> usize {
        let tally = self.tally();
        tally.breakage
            + tally.held_back
            + match strict {
                true => tally.advisory + tally.warned,
                false => 0,
            }
    }

    pub fn findings(&self) -> impl Iterator<Item = &CheckFinding> {
        self.items.iter().flat_map(|item| item.findings.iter())
    }
}

/// Both passes over every item the fixed layout dirs hold.
pub fn check(sealed: &SealedSource) -> Result<CatalogCheck> {
    let mut report = CatalogCheck::default();
    for (kind, dir) in KIND_DIRS {
        let root = sealed.root().join(dir);
        if !sealed.is_dir(&root) {
            continue;
        }
        for entry in sealed.list_dir(&root)? {
            let Some(name) = item_name(&entry) else {
                continue;
            };
            if kind != ItemKind::Skill && !sealed.is_file(&entry) {
                continue;
            }
            report.items.push(check_item(sealed, kind, &name, &entry)?);
        }
    }
    Ok(report)
}

/// Both passes over one item at its catalog path — the unit the indexer
/// scores packages with.
pub fn check_item(
    sealed: &SealedSource,
    kind: ItemKind,
    name: &str,
    path: &Path,
) -> Result<CheckedItem> {
    let content = content(sealed, kind, path)?;
    let file = path
        .strip_prefix(sealed.root())
        .unwrap_or(path)
        .display()
        .to_string();
    let mut findings = structural(kind, name, &file, &content);
    let (verdict, score) = safety(kind, name, &file, content, &mut findings);
    Ok(CheckedItem {
        kind,
        name: name.to_owned(),
        file,
        findings,
        verdict,
        score,
    })
}

/// A skill's whole tree; anything else is one file. A repo-root skill's
/// tree is the repository itself, whose VCS internals and dependency dirs
/// are not content.
fn content(sealed: &SealedSource, kind: ItemKind, path: &Path) -> Result<Content> {
    if kind != ItemKind::Skill {
        return Ok(Content::Document {
            text: sealed.read_to_string(path)?,
        });
    }
    if !sealed.is_dir(path) {
        return Ok(Content::Unread {
            why: "a skill is a directory holding SKILL.md",
        });
    }
    let skip: &[&str] = match path == sealed.root() {
        true => &[".git", "node_modules", "target", "dist", "build", ".venv"],
        false => &[],
    };
    Ok(Content::SkillTree {
        files: sealed
            .collect_tree(path, skip)?
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
/// because a name is carried through untouched; a plugin-registry name is
/// checked by its leaf, since the plugin segment never becomes a filename.
/// A skill tree is checked once for the things its SKILL.md must say — that
/// it exists, that it names the directory it sits in, that it has a
/// description — and it is deliberately *not* checked against the tightest
/// body cap, because rendering splits an oversized skill into `references/`
/// before it reaches the tool that has that cap. Reporting it here would
/// name a problem the renderer has already solved and send an author off to
/// fix something that is not broken.
fn structural(kind: ItemKind, name: &str, file: &str, content: &Content) -> Vec<CheckFinding> {
    let leaf = crate::names::split(name).map_or(name, |(_, leaf)| leaf);
    let mut out = Vec::new();
    for harness in HarnessId::ALL {
        if !crate::harness::capabilities(harness, kind).install.global {
            continue;
        }
        let mut findings = validate::validate_name(harness, leaf);
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
            findings.extend(validate::validate_skill_tree(harness, leaf, leaf, &files));
        }
        out.extend(findings.into_iter().map(|finding| CheckFinding {
            file: file.to_owned(),
            kind: kind.name(),
            name: name.to_owned(),
            pass: harness.name().to_owned(),
            severity: match finding.is_breakage() {
                true => "error",
                false => "warning",
            },
            rule: None,
            message: finding.message,
            fix: finding.remediation,
        }));
    }
    out
}

fn safety(
    kind: ItemKind,
    name: &str,
    file: &str,
    content: Content,
    findings: &mut Vec<CheckFinding>,
) -> (Verdict, u32) {
    let result = quality::audit(AuditInput {
        kind,
        name: name.to_owned(),
        harness: None,
        location: file.to_owned(),
        content,
    });
    let thresholds = quality::Thresholds::default();
    let (verdict, _) = quality::verdict(&result.findings, &result.safety, thresholds);
    findings.extend(result.findings.into_iter().map(|finding| CheckFinding {
        file: finding.location,
        kind: kind.name(),
        name: name.to_owned(),
        pass: SAFETY_PASS.to_owned(),
        severity: finding.severity.name(),
        rule: Some(finding.rule),
        message: finding.message,
        fix: finding.remediation,
    }));
    (verdict, result.safety.score)
}
