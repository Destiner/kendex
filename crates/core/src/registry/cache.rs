//! The directory on disk: one body, one line of metadata. Within the TTL
//! the network is never touched; past it a conditional GET revalidates
//! for the cost of a 304; with the network away the last fetch is served
//! and labeled stale — the Community tab is never blank because a train
//! has no wifi.

use crate::clock;
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::{atomic_write, read_if_exists};
use crate::registry::index::{self, DirectoryIndex};
use crate::registry::{Fetch, base_url};
use serde::{Deserialize, Serialize};

pub const DEFAULT_TTL_SECS: u64 = 3600;

pub struct DirectoryLoad {
    pub index: DirectoryIndex,
    /// When the served body was actually fetched — the "as of" the UI
    /// shows when `stale` is true.
    pub fetched_at: u64,
    /// The network could not be asked and this is the last good fetch.
    pub stale: bool,
}

#[derive(Serialize, Deserialize)]
struct Meta {
    etag: Option<String>,
    fetched_at: u64,
}

/// Read the directory: disk within the TTL, a conditional GET past it,
/// the stale copy when the network fails, an error only with nothing to
/// serve at all.
pub fn load(env: &Env, fetch: &dyn Fetch, force_refresh: bool) -> Result<DirectoryLoad> {
    let cached = read_cached(env);
    let now = clock::unix_now();
    if let Some((meta, index)) = &cached
        && !force_refresh
        && now.saturating_sub(meta.fetched_at) < DEFAULT_TTL_SECS
    {
        return Ok(DirectoryLoad {
            index: index.clone(),
            fetched_at: meta.fetched_at,
            stale: false,
        });
    }

    let etag = cached.as_ref().and_then(|(meta, _)| meta.etag.clone());
    let url = format!("{}/api/v1/index", base_url());
    match fetch.get(&url, etag.as_deref()) {
        Ok(response) if response.status == 304 => {
            let (meta, index) = cached.ok_or_else(|| CoreError::RegistryMalformed {
                why: "the server said 'unchanged' but nothing is cached".into(),
            })?;
            write_meta(
                env,
                &Meta {
                    etag: meta.etag,
                    fetched_at: now,
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
                let dir = env.registry_cache_dir();
                std::fs::create_dir_all(&dir).map_err(|error| CoreError::io(&dir, error))?;
                atomic_write(
                    &dir.join("index.json"),
                    &String::from_utf8_lossy(&response.body),
                )?;
                write_meta(
                    env,
                    &Meta {
                        etag: response.etag,
                        fetched_at: now,
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

fn stale_or(cached: Option<(Meta, DirectoryIndex)>, error: CoreError) -> Result<DirectoryLoad> {
    match cached {
        Some((meta, index)) => Ok(DirectoryLoad {
            index,
            fetched_at: meta.fetched_at,
            stale: true,
        }),
        None => Err(error),
    }
}

fn read_cached(env: &Env) -> Option<(Meta, DirectoryIndex)> {
    let dir = env.registry_cache_dir();
    let meta: Meta =
        serde_json::from_str(&read_if_exists(&dir.join("index.meta.json")).ok()??).ok()?;
    let body = read_if_exists(&dir.join("index.json")).ok()??;
    let index = index::parse(body.as_bytes()).ok()?;
    Some((meta, index))
}

fn write_meta(env: &Env, meta: &Meta) -> Result<()> {
    let dir = env.registry_cache_dir();
    std::fs::create_dir_all(&dir).map_err(|error| CoreError::io(&dir, error))?;
    let json = serde_json::to_string(meta).map_err(|error| CoreError::RegistryMalformed {
        why: error.to_string(),
    })?;
    atomic_write(&dir.join("index.meta.json"), &json)
}
