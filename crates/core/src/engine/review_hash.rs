//! The bytes a decision is about.
//!
//! `content_hash` names what the rules read, and the rules read a *reduced*
//! representation: a skill tree stops after 512 KiB or 200 files, symlinks
//! are stepped over, a binary asset contributes its path and its byte count
//! and nothing else, and text is decoded lossily so two different invalid
//! bytes collapse into one replacement character. That is the right input
//! for scoring and the wrong one for a decision. A plugin whose only file is
//! `payload.wasm` reduces to nothing at all: swap the payload for different
//! bytes of the same length and the representation, the findings and the
//! hash are all unchanged, so a recorded decision goes on speaking for
//! content nobody reviewed.
//!
//! This is the other hash. Every owned byte, or the exact config entry, with
//! no budget and no decoding. A decision binds to it, and the flag that
//! grants one carries it. Where the bytes cannot be reached at all the
//! answer is `None`: a decision with nothing to compare against must never
//! read as live, which is the same rule that reports an artifact vstack
//! cannot compare as uncompared rather than as passing.
//!
//! A hook is the one kind whose two paths see different things. The gate
//! reads the script this plan would write; the scanner finds the hook as a
//! registration inside a shared settings file and never attributes the
//! script to it. Both bind the registration, so the gate's own decision is
//! exact; the observed reading of that registration is what the audit page
//! can compare against, and it is deliberately not the settings file's other
//! keys.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::configedit::ConfigEdit;
use crate::hash::{hash_bytes, hash_files, hash_tree};
use crate::model::{ItemKind, ObservedItem};

use super::desired::{Artifact, Desired};

/// What this plan would install, hashed before a byte of it is written.
pub(super) fn desired(item: &Desired) -> Option<String> {
    let inner = match &item.artifact {
        Artifact::File { bytes, .. } => hash_bytes(bytes),
        Artifact::Tree { files, .. } => hash_files(files),
        Artifact::Registration { script, edits } => registration(script.as_ref(), edits)?,
    };
    Some(seal(item.kind, &inner))
}

/// What is installed here right now, read back off disk.
pub(super) fn observed(item: &ObservedItem) -> Option<String> {
    let inner = match item.kind {
        // The whole tree, every byte of it, and a link inside one is read
        // through rather than skipped — a decision covers what the harness
        // would load, and that is what is at the end of the link.
        ItemKind::Skill | ItemKind::Plugin => match item.path.is_dir() {
            true => hash_tree(&item.path).ok()?,
            false => return None,
        },
        ItemKind::Agent | ItemKind::Command | ItemKind::PiExtension => {
            hash_bytes(&std::fs::read(&item.path).ok()?)
        }
        ItemKind::Hook => hash_bytes(observed_hook(&item.path, &item.name)?.as_bytes()),
        ItemKind::McpServer => hash_bytes(
            canonical(&crate::quality::observe::mcp_entry(&item.path, &item.name)?).as_bytes(),
        ),
    };
    Some(seal(item.kind, &inner))
}

/// The kind is folded in so no two kinds' material can be the same string.
fn seal(kind: ItemKind, inner: &str) -> String {
    hash_bytes(format!("{}|{inner}", kind.name()).as_bytes())
}

/// An entry inside shared harness config: the backing script's bytes, the
/// registration itself, or both. `None` where the plan writes neither — a
/// plugin is one switch in a settings file and a removal has no entry at
/// all, so there is nothing for a decision to bind to.
fn registration(
    script: Option<&(PathBuf, Vec<u8>)>,
    edits: &[(PathBuf, ConfigEdit)],
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some((_, bytes)) = script {
        parts.push(hash_bytes(bytes));
    }
    for (_, edit) in edits {
        match edit {
            ConfigEdit::UpsertHook {
                event,
                matcher,
                command,
                timeout,
            }
            | ConfigEdit::UpsertCopilotHook {
                event,
                matcher,
                command,
                timeout,
            } => parts.push(hook_entry(event, matcher.as_deref(), command, *timeout)),
            ConfigEdit::UpsertMcpServer { value, .. } => parts.push(canonical(value)),
            _ => {}
        }
    }
    match parts.is_empty() {
        true => None,
        false => Some(hash_bytes(parts.join("|").as_bytes())),
    }
}

/// One hook registration as the four values a harness loads it by. An empty
/// matcher is `*`, which is how the scanner names it too — the two readings
/// have to spell one registration the same way.
fn hook_entry(event: &str, matcher: Option<&str>, command: &str, timeout: Option<u32>) -> String {
    let matcher = matcher.filter(|m| !m.is_empty()).unwrap_or("*");
    let timeout = timeout.map(|t| t.to_string()).unwrap_or_default();
    format!("{event}|{matcher}|{command}|{timeout}")
}

/// The registration this observed hook was named after, found again in the
/// config file that holds it. The name is `event:matcher:stem`, so the walk
/// that produced it is the walk that finds it.
fn observed_hook(path: &Path, name: &str) -> Option<String> {
    let root = crate::quality::observe::config_json(path)?;
    let events = root.get("hooks")?.as_object()?;
    for (event, groups) in events {
        for group in groups.as_array()? {
            let matcher = group.get("matcher").and_then(Value::as_str);
            let handlers = match group.get("hooks").and_then(|h| h.as_array()) {
                Some(list) => list.iter().collect::<Vec<_>>(),
                None => vec![group],
            };
            for handler in handlers {
                let Some(command) = handler.get("command").and_then(Value::as_str) else {
                    continue;
                };
                let entry = hook_entry(
                    event,
                    matcher,
                    command,
                    handler
                        .get("timeout")
                        .and_then(Value::as_u64)
                        .and_then(|t| u32::try_from(t).ok()),
                );
                let stem = crate::hook::command_stem(command);
                if name == format!("{event}:{}:{stem}", matcher.unwrap_or("*")) {
                    return Some(entry);
                }
            }
        }
    }
    None
}

/// `value` as text with object keys in one order. The JSON reader preserves
/// the order it found, so two readings of one entry can serialize
/// differently; a decision must not go stale because somebody moved a key.
fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let body: Vec<String> = pairs
                .into_iter()
                .map(|(key, value)| format!("{}:{}", Value::String(key.clone()), canonical(value)))
                .collect();
            format!("{{{}}}", body.join(","))
        }
        Value::Array(items) => {
            let body: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", body.join(","))
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reader keeps insertion order, so the same entry written two ways
    /// must still hash the same — a moved key is not a content change.
    #[test]
    fn key_order_does_not_change_an_entry() {
        let first: Value =
            serde_json::from_str(r#"{"command":"node","args":["a"],"env":{"B":"2","A":"1"}}"#)
                .unwrap();
        let second: Value =
            serde_json::from_str(r#"{"env":{"A":"1","B":"2"},"args":["a"],"command":"node"}"#)
                .unwrap();
        assert_eq!(canonical(&first), canonical(&second));
    }

    /// And a value that actually moved is a different entry.
    #[test]
    fn a_changed_value_changes_an_entry() {
        let first: Value = serde_json::from_str(r#"{"args":["a"]}"#).unwrap();
        let second: Value = serde_json::from_str(r#"{"args":["b"]}"#).unwrap();
        assert_ne!(canonical(&first), canonical(&second));
    }
}
