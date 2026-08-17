//! The safety gate, run over what a plan would write.
//!
//! An item that is not installed yet has nothing to observe, so the only
//! bytes that can gate a fresh install are the ones the renderers just
//! produced. Every desired installation is audited here, before a single op
//! is planned for it.
//!
//! A blocked item goes down the path a refused rendering already takes: a
//! conflict row naming what was found, nothing installed, and any previous,
//! wider copy moved to the trash. That last part is deliberate — leaving a
//! copy live would keep exactly the content the block exists to stop, and
//! the trash is recoverable.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::configedit::ConfigEdit;
use crate::manifest::Manifest;
use crate::model::{HarnessId, ItemKind, Scope};
use crate::quality::overrides::{self, OverrideState};
use crate::quality::{
    AuditInput, Content, McpEntry, QualityScore, SafetyScore, SkippedRule, Thresholds,
    UNREADABLE_PLUGIN, Verdict,
};

use super::PlanOptions;
use super::desired::{Artifact, Desired, DesiredState, Refused};

/// One installation's two scores and everything behind them. Safety and
/// quality sit side by side and are never combined: one answers whether the
/// content is dangerous, the other whether it is any good, and averaging
/// them would let a well-written attack outscore a clumsy honest skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ItemSafety {
    pub kind: ItemKind,
    pub name: String,
    pub harness: HarnessId,
    pub scope: Scope,
    pub safety: SafetyScore,
    /// Advisory only, and absent for kinds with no authored prose.
    pub quality: Option<QualityScore>,
    pub findings: Vec<crate::quality::Finding>,
    /// Rules that apply to this kind but had no bytes to read here.
    pub skipped: Vec<SkippedRule>,
    pub verdict: Verdict,
    /// Why the verdict is what it is, in sentences.
    pub reasons: Vec<String>,
    /// The identity of the bytes the rules read — the reduced input the
    /// findings came out of, budgets and lossy decoding included.
    pub content_hash: String,
    /// The identity of the complete bytes, or of the exact config entry. A
    /// decision binds to this and the flag that grants one carries it, so it
    /// is what a reviewer is accepting. `None` where the bytes cannot be
    /// reached from here at all.
    pub review_hash: Option<String>,
    #[serde(rename = "override")]
    pub override_state: OverrideState,
}

impl ItemSafety {
    /// Whether this item is held back from installing.
    pub fn blocked(&self) -> bool {
        self.verdict == Verdict::Block && !self.override_state.unblocks()
    }
}

/// Audit every desired installation, hold back the ones that fail, and
/// record the overrides this run was asked to grant.
pub(super) fn run(
    scope: &Scope,
    manifest: &Manifest,
    options: &PlanOptions,
    thresholds: Thresholds,
    state: &mut DesiredState,
) -> Vec<ItemSafety> {
    let mut safety = Vec::new();
    let mut kept = Vec::new();
    for item in std::mem::take(&mut state.items) {
        let input = input_for(&item);
        let root = input.location.clone();
        // The override binds to what the rules read, not to what lands on
        // disk. For an MCP server or a plugin those differ: the artifact is
        // a config edit whose backing file may be empty, so hashing the
        // artifact would give every such item the same hash and an override
        // would survive a command line being rewritten under it.
        let content_hash = content_hash(&input);
        let review_hash = super::review_hash::desired(&item);
        let result = crate::quality::audit(input);
        let (verdict, reasons) =
            crate::quality::verdict(&result.findings, &result.safety, thresholds);
        let mut recorded = manifest.safety_overrides.get(&item.key);
        if let Some(review_hash) = &review_hash
            && verdict == Verdict::Block
            && granted(options, &item, review_hash)
        {
            let minted = overrides::mint(review_hash, &result.findings, &root, None);
            let updated = state
                .manifest_update
                .get_or_insert_with(|| manifest.clone());
            updated.safety_overrides.insert(item.key.clone(), minted);
            recorded = updated.safety_overrides.get(&item.key);
        }
        let override_state =
            overrides::state(recorded, review_hash.as_deref(), &result.findings, &root);
        let row = ItemSafety {
            kind: item.kind,
            name: item.name.clone(),
            harness: item.harness,
            scope: scope.clone(),
            safety: result.safety,
            quality: result.quality,
            findings: result.findings,
            skipped: result.skipped,
            verdict,
            reasons,
            content_hash,
            review_hash,
            override_state,
        };
        match row.blocked() {
            true => state.refused.push(refusal(&item, &row)),
            false => kept.push(item),
        }
        safety.push(row);
    }
    state.items = kept;
    safety
}

/// Whether this run was asked to record a review of *this* content.
///
/// The flag names an installation and the content that was shown with it:
/// `name@<hash>`, where the hash is the one printed beside the findings. A
/// bare name does not grant, and that is the whole point — a name in a
/// shell history, a Makefile or a CI job would re-grant against content
/// nobody has read, which is the standing bypass an override exists to not
/// become. When the content changes the printed hash changes with it, so
/// re-running the same command line blocks again and prints the new one.
fn granted(options: &PlanOptions, item: &Desired, review_hash: &str) -> bool {
    options.allow_unsafe.iter().any(|named| {
        let Some((name, shown)) = named.rsplit_once('@') else {
            return false;
        };
        (name == item.name || name == item.key)
            && shown.len() >= SHOWN_HASH
            && review_hash.starts_with(shown)
    })
}

/// How much of the review hash is printed, and the least a flag may carry.
/// Long enough that nobody types a prefix that matches something else by
/// accident, short enough to copy off a terminal.
pub const SHOWN_HASH: usize = 12;

/// The flag that would grant this exact decision, as the user should type
/// it back.
pub fn allow_unsafe_flag(name: &str, review_hash: &str) -> String {
    format!(
        "{name}@{}",
        &review_hash[..SHOWN_HASH.min(review_hash.len())]
    )
}

fn refusal(item: &Desired, row: &ItemSafety) -> Refused {
    let stale = match &row.override_state {
        OverrideState::Stale { why } => format!(" (the recorded review no longer applies: {why})"),
        _ => String::new(),
    };
    Refused {
        kind: item.kind,
        name: item.name.clone(),
        harness: item.harness,
        reason: format!(
            "held back by the safety check — score {}{stale}: {}",
            row.safety.score,
            row.reasons.join("; ")
        ),
    }
}

/// The identity of the bytes the rules read, so an override that was
/// granted against them stops applying when any of them changes.
pub(super) fn content_hash(input: &AuditInput) -> String {
    // The location deliberately stays out of the material. The override is
    // keyed by installation already, and the two scoring paths read the
    // same bytes at different paths — the gate at the canonical tree, the
    // audit at the harness-native link — so folding the path in would make
    // every accepted symlink-method skill read as edited the moment it
    // lands on disk.
    let mut material = format!("{}|", input.kind.name());
    match &input.content {
        Content::Document { text } => material.push_str(text),
        // Sorted, because a plan builds the tree in render order and a scan
        // reads it back in directory order. The same files are the same
        // content whichever order they arrived in, and an override that
        // survived the install has to still recognise what it reviewed.
        Content::SkillTree { files } => {
            let mut entries: Vec<String> = files
                .iter()
                .map(|file| {
                    format!(
                        "{}:{}:{}\n",
                        file.path.display(),
                        file.bytes,
                        file.text.as_deref().unwrap_or_default()
                    )
                })
                .collect();
            entries.sort();
            material.push_str(&entries.concat());
        }
        Content::Hook {
            event,
            matcher,
            command,
            script,
        } => material.push_str(&format!(
            "{event}|{}|{command}|{}",
            matcher.as_deref().unwrap_or_default(),
            script.as_deref().unwrap_or_default()
        )),
        Content::Mcp(entry) => material.push_str(&format!("{entry:?}")),
        Content::Plugin(sources) => material.push_str(&format!("{sources:?}")),
        Content::Unread { why } => material.push_str(why),
    }
    crate::hash::hash_bytes(material.as_bytes())
}

/// What this item's rendering gives the rules to read.
fn input_for(item: &Desired) -> AuditInput {
    let (location, content) = match &item.artifact {
        Artifact::File { path, bytes } => (
            path.display().to_string(),
            Content::Document {
                text: String::from_utf8_lossy(bytes).into_owned(),
            },
        ),
        // Read through the same budgeted constructor the observed audit
        // uses, so the two paths score and hash one construction.
        Artifact::Tree {
            canonical, files, ..
        } => (
            canonical.display().to_string(),
            Content::SkillTree {
                files: crate::quality::observe::tree_files_from_bytes(files),
            },
        ),
        Artifact::Registration { script, edits } => registration(item, script.as_ref(), edits),
    };
    AuditInput {
        kind: item.kind,
        name: item.name.clone(),
        harness: Some(item.harness),
        location,
        content,
    }
}

type Script = (std::path::PathBuf, Vec<u8>);

fn registration(
    item: &Desired,
    script: Option<&Script>,
    edits: &[(std::path::PathBuf, ConfigEdit)],
) -> (String, Content) {
    let location = script
        .map(|(path, _)| path.display().to_string())
        .or_else(|| edits.first().map(|(path, _)| path.display().to_string()))
        .unwrap_or_else(|| item.name.clone());
    let content = match item.kind {
        ItemKind::McpServer => match mcp_entry(edits) {
            Some(entry) => Content::Mcp(entry),
            // A disabled server is planned as a removal, so the plan holds
            // no entry to read and nothing about it can be judged.
            None => Content::Unread {
                why: "this server is being removed from the harness's configuration, not written to it",
            },
        },
        ItemKind::Plugin => Content::Unread {
            why: UNREADABLE_PLUGIN,
        },
        _ => Content::Hook {
            event: String::new(),
            matcher: None,
            command: location.clone(),
            script: script.map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned()),
        },
    };
    (location, content)
}

/// The server entry this plan would write, taken from the config edit that
/// writes it — command, arguments, environment, headers and url, exactly as
/// the harness will store them.
fn mcp_entry(edits: &[(std::path::PathBuf, ConfigEdit)]) -> Option<McpEntry> {
    edits
        .iter()
        .find_map(|(_, edit)| match edit {
            ConfigEdit::UpsertMcpServer { value, .. } => Some(value),
            _ => None,
        })
        .map(McpEntry::from_json)
}
