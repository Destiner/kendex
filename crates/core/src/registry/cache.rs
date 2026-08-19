//! The directory on disk: one file holding body, ETag and fetch time as a
//! single generation — written atomically, so no crash can pair one
//! fetch's body with another's ETag. Within the TTL the network is never
//! touched; past it a conditional GET revalidates for the cost of a 304;
//! with the network away the last fetch is served and labeled stale — the
//! Community tab is never blank because a train has no wifi.

use crate::clock;
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::{atomic_write, read_if_exists};
use crate::registry::index::{self, DirectoryIndex};
use crate::registry::{Fetch, MAX_RESPONSE_BYTES, base_url};
use serde::{Deserialize, Serialize};

pub const DEFAULT_TTL_SECS: u64 = 3600;
const CACHE_FILE: &str = "index.cache.json";

pub struct DirectoryLoad {
    pub index: DirectoryIndex,
    /// When the served body was actually fetched — the "as of" the UI
    /// shows when `stale` is true.
    pub fetched_at: u64,
    /// The network could not be asked and this is the last good fetch.
    pub stale: bool,
}

/// One fetch, whole: the ETag belongs to exactly this body because they
/// are one write.
#[derive(Serialize, Deserialize)]
struct Generation {
    etag: Option<String>,
    fetched_at: u64,
    body: String,
}

/// Read the directory: disk within the TTL, a conditional GET past it,
/// the stale copy when the network fails, an error only with nothing to
/// serve at all.
pub fn load(env: &Env, fetch: &dyn Fetch, force_refresh: bool) -> Result<DirectoryLoad> {
    let cached = read_cached(env);
    let now = clock::unix_now();
    if let Some((generation, index)) = &cached
        && !force_refresh
        && now.saturating_sub(generation.fetched_at) < DEFAULT_TTL_SECS
    {
        return Ok(DirectoryLoad {
            index: index.clone(),
            fetched_at: generation.fetched_at,
            stale: false,
        });
    }

    let etag = cached
        .as_ref()
        .and_then(|(generation, _)| generation.etag.clone());
    let url = format!("{}/api/v1/index", base_url());
    match fetch.get(&url, etag.as_deref()) {
        Ok(response) if response.status == 304 => {
            let (generation, index) = cached.ok_or_else(|| CoreError::RegistryMalformed {
                why: "the server said 'unchanged' but nothing is cached".into(),
            })?;
            write_generation(
                env,
                &Generation {
                    fetched_at: now,
                    ..generation
                },
            )?;
            Ok(DirectoryLoad {
                index,
                fetched_at: now,
                stale: false,
            })
        }
        Ok(response) if response.status == 200 => match index::parse(&response.body) {
            Ok(index) => {
                write_generation(
                    env,
                    &Generation {
                        etag: response.etag,
                        fetched_at: now,
                        body: String::from_utf8_lossy(&response.body).into_owned(),
                    },
                )?;
                Ok(DirectoryLoad {
                    index,
                    fetched_at: now,
                    stale: false,
                })
            }
            Err(error) => stale_or(cached, error),
        },
        Ok(response) => stale_or(
            cached,
            CoreError::RegistryUnavailable {
                why: format!("the directory answered {}", response.status),
            },
        ),
        Err(error) => stale_or(cached, error),
    }
}

fn stale_or(
    cached: Option<(Generation, DirectoryIndex)>,
    error: CoreError,
) -> Result<DirectoryLoad> {
    match cached {
        Some((generation, index)) => Ok(DirectoryLoad {
            index,
            fetched_at: generation.fetched_at,
            stale: true,
        }),
        None => Err(error),
    }
}

fn read_cached(env: &Env) -> Option<(Generation, DirectoryIndex)> {
    let path = env.registry_cache_dir().join(CACHE_FILE);
    // The cache lives on this machine, but "on this machine" is not
    // "trusted to be well-formed": a replaced or corrupt file must not be
    // read past the same cap the network honors, and must re-pass the
    // same strict parse.
    let size = std::fs::metadata(&path).ok()?.len();
    if size > MAX_RESPONSE_BYTES as u64 * 2 {
        return None;
    }
    let generation: Generation = serde_json::from_str(&read_if_exists(&path).ok()??).ok()?;
    let index = index::parse(generation.body.as_bytes()).ok()?;
    Some((generation, index))
}

fn write_generation(env: &Env, generation: &Generation) -> Result<()> {
    let dir = env.registry_cache_dir();
    std::fs::create_dir_all(&dir).map_err(|error| CoreError::io(&dir, error))?;
    let json = serde_json::to_string(generation).map_err(|error| CoreError::RegistryMalformed {
        why: error.to_string(),
    })?;
    atomic_write(&dir.join(CACHE_FILE), &json)
}
