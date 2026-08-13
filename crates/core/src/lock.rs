use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::{atomic_write, read_if_exists};
use crate::manifest::Method;
use crate::model::{HarnessId, ItemKind, Scope};

/// Current lock version. Version 1 (v0.1) still loads — the shape is
/// compatible and the next lock write records the current version. A lock
/// newer than this build refuses to load.
pub const LOCK_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
pub struct Lock {
    pub version: u32,
    #[serde(default)]
    pub entries: BTreeMap<String, LockEntry>,
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
    pub enabled: bool,
    /// Agents only: the source's skill set at last sync, so upstream
    /// additions merge in while user removals stay durable — deterministic
    /// across cache loss and machines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_skills: Option<Vec<String>>,
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

pub fn load(path: &Path) -> Result<Lock> {
    match read_if_exists(path)? {
        None => Ok(Lock {
            version: LOCK_VERSION,
            entries: BTreeMap::new(),
        }),
        Some(text) => {
            let lock: Lock = serde_json::from_str(&text).map_err(|e| CoreError::JsonParse {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;
            if lock.version > LOCK_VERSION {
                return Err(CoreError::SchemaTooNew {
                    path: path.to_path_buf(),
                    found: i64::from(lock.version),
                });
            }
            Ok(lock)
        }
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

/// Now, as an ISO-8601 UTC timestamp (no external time dependency).
pub fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    let rem = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's civil-from-days: days since 1970-01-01 → (y, m, d).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
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
            entries: BTreeMap::new(),
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
                installed_at: timestamp(),
                source_hash: "abc".into(),
                enabled: true,
                upstream_skills: None,
            },
        );
        save(&path, &lock).unwrap();
        assert_eq!(load(&path).unwrap(), lock);
        assert!(std::fs::read_to_string(&path).unwrap().ends_with('\n'));
    }

    #[test]
    fn timestamps_are_iso8601() {
        let ts = timestamp();
        assert_eq!(ts.len(), 20);
        assert!(ts.ends_with('Z'));
        assert!(ts.starts_with("20"));
    }
}
