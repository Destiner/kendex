//! Skills.sh search behind a versioned adapter. Their API is not a
//! contract: the schema is pinned to what was observed, anything that
//! stops matching is refused rather than guessed at, and the kill switch
//! hides the surface entirely. A result row is a lead, never an identity —
//! installs bind to what kendex's own discovery finds in the repository.

use crate::error::{CoreError, Result};
use crate::registry::Fetch;
use serde::{Deserialize, Serialize};

/// Bump when the pinned wire schema below changes shape.
pub const ADAPTER_VERSION: u32 = 1;
const MAX_RESULTS: usize = 50;
const MAX_QUERY: usize = 100;

/// The kill switch: exported so every surface (tab, CLI) agrees. Off
/// hides the section without touching ordinary marketplaces.
pub fn enabled() -> bool {
    std::env::var("KENDEX_SKILLSSH").map_or(true, |value| value != "off")
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SkillsShHit {
    /// The skill's directory name inside its repository.
    pub skill: String,
    /// `owner/repo` — what an install actually subscribes to.
    pub repo: String,
    pub installs: u32,
}

#[derive(Deserialize)]
struct WireSearch {
    skills: Vec<WireSkill>,
}

#[derive(Deserialize)]
struct WireSkill {
    name: String,
    source: String,
    installs: Option<u64>,
}

/// Search skills.sh. Public, unauthenticated, direct — only skills.sh
/// sees the query, and the About text says so.
pub fn search(fetch: &dyn Fetch, query: &str) -> Result<Vec<SkillsShHit>> {
    if !enabled() {
        return Err(CoreError::RegistryUnavailable {
            why: "skills.sh is switched off (KENDEX_SKILLSSH=off)".into(),
        });
    }
    let trimmed: String = query.trim().chars().take(MAX_QUERY).collect();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let url = format!(
        "https://skills.sh/api/search?q={}&limit={MAX_RESULTS}",
        urlencode(&trimmed)
    );
    let response = fetch.get(&url, None)?;
    if response.status != 200 {
        return Err(CoreError::RegistryUnavailable {
            why: format!("skills.sh answered {}", response.status),
        });
    }
    let wire: WireSearch =
        serde_json::from_slice(&response.body).map_err(|error| CoreError::RegistryMalformed {
            why: format!("skills.sh search (adapter v{ADAPTER_VERSION}): {error}"),
        })?;
    Ok(wire
        .skills
        .into_iter()
        .take(MAX_RESULTS)
        .filter_map(|hit| {
            let repo = hit.source;
            let (owner, name) = repo.split_once('/')?;
            // Every part must survive as one URL path segment: the hit
            // becomes `skills.sh/owner/repo/skill`, and a name a separator
            // or control byte could smuggle through is not offered at all
            // — a row whose Install cannot work is worse than no row.
            if !component_ok(owner) || !component_ok(name) || !component_ok(&hit.name) {
                return None;
            }
            Some(SkillsShHit {
                skill: hit.name,
                repo,
                installs: hit.installs.unwrap_or(0).min(u32::MAX as u64) as u32,
            })
        })
        .collect())
}

fn component_ok(part: &str) -> bool {
    !part.is_empty()
        && part.len() <= 120
        && part != ".."
        && part != "."
        && part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn urlencode(text: &str) -> String {
    let mut encoded = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
