//! Per-mirror fetch stamps: when a mirror was last brought current, what
//! its refs looked like then, and — monotonically — when it first started
//! failing. The stamps live beside the bare mirrors and are derived,
//! machine-local state: losing one costs a refetch, nothing else.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::env::Env;
use crate::error::Result;
use crate::fs::{atomic_write, read_if_exists};
use crate::process::Hardened;

/// How old a successful fetch may be before the mirror counts as stale and
/// the background refresh picks it up.
pub const TTL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FetchStamp {
    /// Unix seconds of the last successful fetch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<u64>,
    /// Digest of the mirror's refs after that fetch — what a package
    /// snapshot's evaluation is compared against to detect a mirror that
    /// moved since it was judged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refs_state: Option<String>,
    /// Unix seconds when fetching first started failing. Monotonic: later
    /// failures keep the earliest time, and only a success clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_failure_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl FetchStamp {
    /// Whether the mirror needs a fetch. Never fetched is stale; so is a
    /// stamp from the future — a clock that ran backwards must trigger a
    /// refresh, not certify freshness until it catches up.
    pub fn is_stale(&self, now: u64) -> bool {
        match self.fetched_at {
            None => true,
            Some(at) => at > now || now.saturating_sub(at) > TTL.as_secs(),
        }
    }

    /// A failure old enough to be drift in its own right: twice the TTL,
    /// dated from the first failure, so a flaky hour never nags but a dead
    /// source does not stay quiet forever.
    pub fn failing_since(&self, now: u64) -> Option<u64> {
        let first = self.first_failure_at?;
        (now.saturating_sub(first) > 2 * TTL.as_secs()).then_some(first)
    }
}

pub fn stamp_path(env: &Env, key: &str) -> PathBuf {
    env.source_cache_dir()
        .join("stamps")
        .join(format!("{key}.json"))
}

/// Absent or unreadable reads as never-fetched: the conservative state,
/// which triggers a refresh rather than certifying anything.
pub fn load(env: &Env, key: &str) -> FetchStamp {
    read_if_exists(&stamp_path(env, key))
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn store(env: &Env, key: &str, stamp: &FetchStamp) -> Result<()> {
    let text = serde_json::to_string_pretty(stamp).unwrap_or_default();
    atomic_write(&stamp_path(env, key), &text)
}

pub fn record_success(env: &Env, key: &str, refs_state: Option<String>, now: u64) -> Result<()> {
    store(
        env,
        key,
        &FetchStamp {
            fetched_at: Some(now),
            refs_state,
            first_failure_at: None,
            last_error: None,
        },
    )
}

/// Failures keep the earliest first-failure time — the drift line dates
/// from when the source went dark, not from the latest attempt. A recorded
/// first failure in the future is a clock artifact and resets to now.
pub fn record_failure(env: &Env, key: &str, error: &str, now: u64) -> Result<()> {
    let mut stamp = load(env, key);
    stamp.first_failure_at = match stamp.first_failure_at {
        Some(first) if first <= now => Some(first),
        _ => Some(now),
    };
    stamp.last_error = Some(error.to_owned());
    store(env, key, &stamp)
}

/// Digest of every ref the mirror holds — the identity a package snapshot
/// records as "what I was evaluated against". `None` when the mirror
/// cannot answer, which downstream reads as "cannot compare".
pub fn refs_state(mirror: &Path) -> Option<String> {
    let output = Hardened::git_bare(
        mirror,
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    )
    .run()
    .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(crate::hash::hash_bytes(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    #[test]
    fn stamp_round_trip_and_staleness() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let now = 1_000_000_000;

        assert!(load(&env, "repo-x").is_stale(now));
        record_success(&env, "repo-x", Some("abc".into()), now).unwrap();
        let stamp = load(&env, "repo-x");
        assert!(!stamp.is_stale(now));
        assert!(!stamp.is_stale(now + TTL.as_secs()));
        assert!(stamp.is_stale(now + TTL.as_secs() + 1));
        assert_eq!(stamp.refs_state.as_deref(), Some("abc"));
    }

    #[test]
    fn a_stamp_from_the_future_is_stale() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        record_success(&env, "repo-x", None, 2_000).unwrap();
        assert!(load(&env, "repo-x").is_stale(1_000));
    }

    #[test]
    fn first_failure_is_monotonic_and_cleared_by_success() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        record_failure(&env, "repo-x", "offline", 100).unwrap();
        record_failure(&env, "repo-x", "still offline", 500).unwrap();
        let stamp = load(&env, "repo-x");
        assert_eq!(stamp.first_failure_at, Some(100));
        assert_eq!(stamp.last_error.as_deref(), Some("still offline"));
        assert_eq!(stamp.failing_since(100 + 2 * TTL.as_secs()), None);
        assert_eq!(
            stamp.failing_since(101 + 2 * TTL.as_secs()),
            Some(100),
            "an old enough failure becomes drift, dated from the first one"
        );

        record_success(&env, "repo-x", None, 600).unwrap();
        assert_eq!(load(&env, "repo-x").first_failure_at, None);
    }

    #[test]
    fn corrupt_stamp_reads_as_never_fetched() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let path = stamp_path(&env, "repo-x");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        assert!(load(&env, "repo-x").is_stale(0));
    }
}
