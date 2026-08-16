use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::{atomic_write, read_if_exists};
use crate::manifest::Method;
use crate::model::{HarnessId, ItemKind, Scope};

/// Current lock version. Versions 1 (v0.1) and 2 still load — the shapes
/// are compatible and the next lock write records the current version. A
/// lock newer than this build refuses to load. Version 3 added
/// `source_commit` and `rendered_hash`; the bump is what stops an older
/// build from reading the lock, dropping both on its next write, and
/// erasing the record that tells an edit from an update.
pub const LOCK_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
pub struct Lock {
    pub version: u32,
    #[serde(default)]
    pub entries: BTreeMap<String, LockEntry>,
    /// The commit each declared source resolved to, by source name.
    /// Reproducibility cache, never intent: the manifest says which
    /// revision is wanted, this says which commit that came out as. A lost
    /// lock costs the record, not the pin.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, SourceRev>,
}

/// One source's resolution at the last write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceRev {
    /// `owner/repo`, a canonical path, or `local`.
    pub repo: String,
    /// The selector that produced it, when the manifest names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub commit: String,
}

/// One installation an edge points at: the counterpart named the way the
/// manifest and the lock name an installation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallRef {
    /// Declared source name — dependencies stay inside one catalog, so this
    /// is the source both ends share.
    pub source: String,
    pub kind: ItemKind,
    pub name: String,
    pub harness: HarnessId,
    pub scope: Scope,
}

/// The bundle an installation came in with.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BundleRef {
    pub source: String,
    pub name: String,
    pub scope: Scope,
}

/// Why one installation exists. An installation holds a *set* of these — the
/// user asked for it, two bundles carry it, three items require it — and
/// each is a structured value, never a sentence to parse back.
///
/// The set is a cache, not intent: the manifest records the choices (what
/// was requested, which optional dependencies were taken, what is kept
/// removed) and the plan derives the closure again from those choices plus
/// the catalogs. A lost lock therefore loses nothing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum Reason {
    /// The user asked for this item by name.
    Requested,
    /// Another installed item declares it as a dependency.
    RequiredBy { by: InstallRef },
    /// An installed bundle carries it as a member.
    MemberOf { bundle: BundleRef },
}

/// One installation the engine wrote: item × harness within this scope's
/// lock file. Provenance is durable — a recorded source is never silently
/// rebound (invariant 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LockEntry {
    pub name: String,
    pub kind: ItemKind,
    pub harness: HarnessId,
    /// Declared source name at install time.
    pub source: String,
    /// Resolved provenance: `owner/repo`, a canonical path, or `local`.
    pub source_repo: String,
    pub method: Method,
    pub installed_at: String,
    /// Source bytes + the manifest sections that shaped the artifact.
    pub source_hash: String,
    /// The source commit the bytes came from, for remotes. Cache, like the
    /// rest of the lock: losing it costs the Updates page its "current
    /// version" until the next apply records it again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
    /// What the apply wrote to disk (file/tree artifacts only) — the anchor
    /// that tells a later pass whether the disk moved because upstream did
    /// or because the user edited it. Absent on pre-upgrade entries; the
    /// next apply backfills it, and until then an ambiguous divergence is
    /// reported as a conflict, never overwritten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered_hash: Option<String>,
    pub enabled: bool,
    /// Agents only: the source's skill set at last sync, so upstream
    /// additions merge in while user removals stay durable — deterministic
    /// across cache loss and machines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_skills: Option<Vec<String>>,
    /// What was written, when the harness stores this kind as another one.
    /// Removal and refresh read it instead of deriving a path the install
    /// never took.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emitted: Option<EmittedArtifact>,
    /// Every reason this installation exists. Never empty once written: an
    /// installation nothing can account for would be swept the moment
    /// anything looked at it.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub reasons: BTreeSet<Reason>,
}

/// The artifact one installation actually put on disk, in the harness's own
/// terms: a codex command lands as a skill, under a name the user types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EmittedArtifact {
    pub kind: ItemKind,
    pub name: String,
    pub paths: Vec<PathBuf>,
}

pub fn entry_key(kind: ItemKind, name: &str, harness: HarnessId) -> String {
    format!("{}:{name}:{}", kind.name(), harness.name())
}

pub fn lock_path(env: &Env, scope: &Scope) -> PathBuf {
    match scope {
        Scope::Global => env.global_lock_file(),
        Scope::Project { root } => Env::project_lock_file(root),
    }
}

/// What sits at a lock path. A v1 lock keys entries by bare name and carries
/// a `harnesses` array; v2 keys carry `kind:name:harness` with one `harness`
/// field. The shapes are incompatible — deserializing v1 text straight into
/// [`Lock`] surfaces a raw "missing field" error instead of naming what the
/// file actually is.
#[derive(Debug, Clone, PartialEq)]
pub enum LockFile {
    Absent,
    Legacy { raw: String },
    Current(Lock),
}

/// `"version": 1` alone cannot identify the old product generation: this
/// product's own v0.1 locks also say 1 and load compatibly (see
/// [`LOCK_VERSION`]). The key shape is the discriminator — bare-name keys
/// against v2's always-present `kind:name:harness` separator — and it only
/// applies to files not declaring a version above 1, so a current file with
/// odd keys is diagnosed as corrupt, never mislabeled v1. An empty map
/// matches both shapes and reads as current, the only reading a lost lock
/// or a fresh scope can mean.
fn is_v1(value: &serde_json::Value) -> bool {
    let version = value.get("version").and_then(serde_json::Value::as_i64);
    if version.is_some_and(|v| v > 1) {
        return false;
    }
    value
        .get("entries")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|entries| !entries.is_empty() && entries.keys().all(|k| !k.contains(':')))
}

/// Text-taking wrapper for callers (the CLI's one-shot v1 importer) that
/// have not already parsed the JSON. Unparseable text is not v1 — it is
/// corrupt, and `load_file` names that distinctly.
pub fn is_v1_text(text: &str) -> bool {
    serde_json::from_str(text).is_ok_and(|value| is_v1(&value))
}

pub fn load_file(path: &Path) -> Result<LockFile> {
    let Some(text) = read_if_exists(path)? else {
        return Ok(LockFile::Absent);
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::LockCorrupt {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
    if let Some(version) = value.get("version").and_then(serde_json::Value::as_i64)
        && version > i64::from(LOCK_VERSION)
    {
        return Err(CoreError::SchemaTooNew {
            path: path.to_path_buf(),
            found: version,
        });
    }
    if is_v1(&value) {
        return Ok(LockFile::Legacy { raw: text });
    }
    let mut lock: Lock = serde_json::from_value(value).map_err(|e| CoreError::LockCorrupt {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    backfill_requested_reason(&mut lock);
    Ok(LockFile::Current(lock))
}

// A record written before installations carried their reasons: everything
// installed then was installed because it was asked for, which is the only
// reading that cannot invent a dependency nobody declared. The next write
// records it.
fn backfill_requested_reason(lock: &mut Lock) {
    for entry in lock.entries.values_mut() {
        if entry.reasons.is_empty() {
            entry.reasons.insert(Reason::Requested);
        }
    }
}

/// Load for mutation: a v1 lock is a hard error, never a write target — same
/// posture as [`crate::manifest::file::load_for_mutation`]. Callers that only
/// observe (the audit view) use [`load_file`] instead so a v1 lock degrades
/// to a note rather than blocking the read.
pub fn load(path: &Path) -> Result<Lock> {
    match load_file(path)? {
        LockFile::Absent => Ok(Lock {
            version: LOCK_VERSION,
            ..Lock::default()
        }),
        LockFile::Legacy { .. } => Err(CoreError::LegacyLock {
            path: path.to_path_buf(),
        }),
        LockFile::Current(lock) => Ok(lock),
    }
}

pub fn save(path: &Path, lock: &Lock) -> Result<()> {
    let mut text = serde_json::to_string_pretty(lock).map_err(|e| CoreError::JsonParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    text.push('\n');
    atomic_write(path, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_round_trips_and_missing_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".vstack-lock.json");
        assert_eq!(load(&path).unwrap().entries.len(), 0);

        let mut lock = Lock {
            version: LOCK_VERSION,
            ..Lock::default()
        };
        lock.entries.insert(
            entry_key(ItemKind::Skill, "github", HarnessId::Claude),
            LockEntry {
                name: "github".into(),
                kind: ItemKind::Skill,
                harness: HarnessId::Claude,
                source: "vstack".into(),
                source_repo: "vanillagreencom/vstack".into(),
                method: Method::Symlink,
                installed_at: crate::clock::timestamp(),
                source_hash: "abc".into(),
                source_commit: None,
                rendered_hash: None,
                enabled: true,
                upstream_skills: None,
                emitted: None,
                reasons: BTreeSet::from([
                    Reason::Requested,
                    Reason::RequiredBy {
                        by: InstallRef {
                            source: "vstack".into(),
                            kind: ItemKind::Skill,
                            name: "dev".into(),
                            harness: HarnessId::Claude,
                            scope: Scope::Global,
                        },
                    },
                    Reason::MemberOf {
                        bundle: BundleRef {
                            source: "vstack".into(),
                            name: "starter".into(),
                            scope: Scope::Global,
                        },
                    },
                ]),
            },
        );
        save(&path, &lock).unwrap();
        assert_eq!(load(&path).unwrap(), lock);
        assert!(std::fs::read_to_string(&path).unwrap().ends_with('\n'));
    }

    /// A record from before installations carried reasons reads back as one
    /// the user asked for — the only reading that invents nothing.
    #[test]
    fn entries_without_reasons_read_as_requested() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".vstack-lock.json");
        std::fs::write(
            &path,
            r#"{"version":2,"entries":{"skill:gh:claude":{"name":"gh","kind":"skill","harness":"claude","source":"vstack","sourceRepo":"vanillagreencom/vstack","method":"symlink","installedAt":"2026-01-01T00:00:00Z","sourceHash":"abc","enabled":true}}}"#,
        )
        .unwrap();
        let lock = load(&path).unwrap();
        let entry = &lock.entries["skill:gh:claude"];
        assert_eq!(entry.reasons, BTreeSet::from([Reason::Requested]));
    }

    #[test]
    fn timestamps_are_iso8601() {
        let ts = crate::clock::timestamp();
        assert_eq!(ts.len(), 20);
        assert!(ts.ends_with('Z'));
        assert!(ts.starts_with("20"));
    }

    /// A v1 lock (bare-name keys, a `harnesses` array, no singular
    /// `harness`) must read as a recognized legacy shape, never as a raw
    /// serde "missing field" error.
    #[test]
    fn a_v1_lock_reads_as_legacy_not_a_parse_error() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".vstack-lock.json");
        std::fs::write(
            &path,
            r#"{"version":1,"entries":{"gh":{"name":"gh","kind":"skill","source":"vstack","source_repo":"vanillagreencom/vstack","harnesses":["claude-code"],"method":"symlink","installed_at":"2026-01-01T00:00:00Z","source_hash":"abc"}}}"#,
        )
        .unwrap();
        assert!(matches!(load_file(&path).unwrap(), LockFile::Legacy { .. }));
        assert!(matches!(load(&path), Err(CoreError::LegacyLock { .. })));
    }

    /// Malformed JSON is reported as a damaged lock, distinct from the v1
    /// and future-version cases.
    #[test]
    fn unparseable_json_is_reported_as_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".vstack-lock.json");
        std::fs::write(&path, "{not json").unwrap();
        assert!(matches!(
            load_file(&path),
            Err(CoreError::LockCorrupt { .. })
        ));
        assert!(matches!(load(&path), Err(CoreError::LockCorrupt { .. })));
    }

    /// A lock a future vstack wrote refuses to load rather than being
    /// silently misread or corrupted by an older build.
    #[test]
    fn a_newer_lock_refuses_to_load() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".vstack-lock.json");
        std::fs::write(&path, r#"{"version":99,"entries":{}}"#).unwrap();
        assert!(matches!(
            load_file(&path),
            Err(CoreError::SchemaTooNew { found: 99, .. })
        ));
        assert!(matches!(
            load(&path),
            Err(CoreError::SchemaTooNew { found: 99, .. })
        ));
    }

    /// An empty `entries` map is indistinguishable between v1 and v2 — it
    /// reads as v2, since that is the only reading a fresh scope can mean.
    #[test]
    fn an_empty_lock_reads_as_current() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".vstack-lock.json");
        std::fs::write(&path, r#"{"version":1,"entries":{}}"#).unwrap();
        assert!(matches!(load_file(&path).unwrap(), LockFile::Current(_)));
    }
}
