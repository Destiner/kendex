//! The directory index, parsed strictly. The server enforces these caps
//! at write time; enforcing them again here means a compromised or
//! spoofed registry still cannot grow a row past what every screen and
//! subscriber expects.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

pub const MAX_MARKETPLACES: usize = 500;
pub const MAX_PACKAGES: usize = 512;
pub const MAX_BUNDLES: usize = 64;
pub const MAX_MEMBERS: usize = 128;
pub const MAX_TAGS: usize = 12;
pub const MAX_NAME: usize = 120;
pub const MAX_DESCRIPTION: usize = 400;
pub const MAX_TEXT: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryIndex {
    pub generated_at: Option<String>,
    pub marketplaces: Vec<DirectoryMarketplace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryMarketplace {
    /// `owner/repo` — validated segments, safe to hand to subscribe.
    pub repo: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub featured: bool,
    pub head_commit: Option<String>,
    pub package_count: u32,
    pub bundle_count: u32,
    pub packages: Vec<DirectoryPackage>,
    pub bundles: Vec<DirectoryBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryPackage {
    pub kind: String,
    pub name: String,
    pub description: Option<String>,
    pub safety_score: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryBundle {
    pub name: String,
    pub description: Option<String>,
    pub member_count: u32,
}

/// What the wire carries (the site's schema 1). Kept private: everything
/// leaves this module already validated and capped.
#[derive(Deserialize)]
struct WireIndex {
    schema: u32,
    generated_at: Option<String>,
    marketplaces: Vec<WireMarketplace>,
}

#[derive(Deserialize)]
struct WireMarketplace {
    repo: String,
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    status: Option<String>,
    head_commit: Option<String>,
    counts: Option<WireCounts>,
    #[serde(default)]
    packages: Vec<WirePackage>,
    #[serde(default)]
    bundles: Vec<WireBundle>,
}

#[derive(Deserialize)]
struct WireCounts {
    packages: Option<u32>,
    bundles: Option<u32>,
}

#[derive(Deserialize)]
struct WirePackage {
    kind: String,
    name: String,
    description: Option<String>,
    safety: Option<WireSafety>,
}

#[derive(Deserialize)]
struct WireSafety {
    score: Option<f64>,
}

#[derive(Deserialize)]
struct WireBundle {
    name: String,
    description: Option<String>,
    #[serde(default)]
    members: Vec<serde_json::Value>,
}

/// Parse the index body. Structural problems — bad JSON, a schema this
/// build does not speak — refuse loudly; a single unusable row is dropped
/// rather than sinking the directory.
pub fn parse(body: &[u8]) -> Result<DirectoryIndex> {
    let wire: WireIndex =
        serde_json::from_slice(body).map_err(|error| CoreError::RegistryMalformed {
            why: error.to_string(),
        })?;
    if wire.schema != 1 {
        return Err(CoreError::RegistryMalformed {
            why: format!("index schema {} is not one this build reads", wire.schema),
        });
    }
    let marketplaces = wire
        .marketplaces
        .into_iter()
        .take(MAX_MARKETPLACES)
        .filter_map(from_wire)
        .collect();
    Ok(DirectoryIndex {
        generated_at: wire.generated_at.map(|at| capped(&at, MAX_TEXT)),
        marketplaces,
    })
}

fn from_wire(row: WireMarketplace) -> Option<DirectoryMarketplace> {
    repo_ok(&row.repo)?;
    let packages: Vec<DirectoryPackage> = row
        .packages
        .into_iter()
        .take(MAX_PACKAGES)
        .filter_map(|pkg| {
            if pkg.kind.is_empty() || pkg.name.is_empty() {
                return None;
            }
            Some(DirectoryPackage {
                kind: capped(&pkg.kind, 40),
                name: capped(&pkg.name, MAX_NAME),
                description: pkg.description.map(|text| capped(&text, MAX_DESCRIPTION)),
                safety_score: pkg
                    .safety
                    .and_then(|safety| safety.score)
                    .map(|score| score.clamp(0.0, 100.0) as u8),
            })
        })
        .collect();
    let bundles: Vec<DirectoryBundle> = row
        .bundles
        .into_iter()
        .take(MAX_BUNDLES)
        .filter_map(|bundle| {
            if bundle.name.is_empty() {
                return None;
            }
            Some(DirectoryBundle {
                name: capped(&bundle.name, MAX_NAME),
                description: bundle
                    .description
                    .map(|text| capped(&text, MAX_DESCRIPTION)),
                member_count: bundle.members.len().min(MAX_MEMBERS) as u32,
            })
        })
        .collect();
    let counts = row.counts.as_ref();
    Some(DirectoryMarketplace {
        repo: row.repo,
        name: row.name.map(|name| capped(&name, MAX_NAME)),
        description: row.description.map(|text| capped(&text, MAX_DESCRIPTION)),
        tags: row
            .tags
            .into_iter()
            .take(MAX_TAGS)
            .map(|tag| capped(&tag, 40))
            .collect(),
        featured: row.status.as_deref() == Some("featured"),
        head_commit: row.head_commit.filter(|commit| {
            commit.len() <= 64 && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        }),
        package_count: counts
            .and_then(|c| c.packages)
            .unwrap_or(packages.len() as u32),
        bundle_count: counts
            .and_then(|c| c.bundles)
            .unwrap_or(bundles.len() as u32),
        packages,
        bundles,
    })
}

/// The same segment rule the site enforces; a row that fails it is not a
/// repository the app could subscribe to, so it is not offered.
fn repo_ok(repo: &str) -> Option<()> {
    let (owner, name) = repo.split_once('/')?;
    let segment = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            && part.as_bytes()[0].is_ascii_alphanumeric()
            && part != ".."
    };
    (segment(owner) && segment(name) && !name.contains('/')).then_some(())
}

fn capped(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    match trimmed.char_indices().nth(max) {
        Some((at, _)) => trimmed[..at].to_string(),
        None => trimmed.to_string(),
    }
}
