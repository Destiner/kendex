//! Report routing: which repo an issue about an installed item belongs to.
//! kendex-owned assets file upstream; everything else files against the
//! user's own repo — the safe default. Skills never route upstream via the
//! lock (distribution is not ownership); only their own frontmatter can
//! opt them in.

use crate::env::Env;
use crate::lock::Lock;
use crate::model::{ItemKind, Scope};

pub const DEFAULT_UPSTREAM: &str = "vanillagreencom/vstack";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub kendex_owned: bool,
    /// Upstream `owner/repo` to file against — only when kendex-owned.
    pub repo: Option<String>,
    /// Routing label — only on the canonical upstream, where it exists.
    pub label: Option<String>,
}

/// The routing label for a kendex-owned asset, by what it is.
pub fn derive_label(name: &str, kind: Option<ItemKind>) -> &'static str {
    if name.contains("review-gate") {
        return "ci-infra";
    }
    match kind {
        Some(ItemKind::Hook | ItemKind::PiExtension) => "harness",
        Some(ItemKind::Skill | ItemKind::Agent) => "skills",
        _ => "cli",
    }
}

pub fn route(
    env: &Env,
    scope: &Scope,
    lock: &Lock,
    name: &str,
    kind: Option<ItemKind>,
    upstream: &str,
) -> Route {
    let (fm_source, fm_repo) = installed_frontmatter(env, scope, name);
    let owned = is_kendex_owned(
        lock,
        name,
        kind,
        fm_source.as_deref(),
        fm_repo.as_deref(),
        upstream,
    );
    Route {
        kendex_owned: owned,
        repo: owned.then(|| upstream.to_owned()),
        label: (owned && upstream == DEFAULT_UPSTREAM).then(|| derive_label(name, kind).to_owned()),
    }
}

fn is_kendex_owned(
    lock: &Lock,
    name: &str,
    kind: Option<ItemKind>,
    frontmatter_source: Option<&str>,
    frontmatter_repo: Option<&str>,
    upstream: &str,
) -> bool {
    // Catalog content of both product-name generations opts in.
    if matches!(frontmatter_source, Some("kendex" | "vstack"))
        || frontmatter_repo == Some(DEFAULT_UPSTREAM)
    {
        return true;
    }
    lock.entries.values().any(|entry| {
        entry.name == name
            && kind.is_none_or(|k| k == entry.kind)
            && entry.kind != ItemKind::Skill
            && entry.source_repo == upstream
    })
}

/// `source:`/`repository:` from the installed skill's frontmatter — the one
/// place a skill can claim kendex ownership.
fn installed_frontmatter(env: &Env, scope: &Scope, name: &str) -> (Option<String>, Option<String>) {
    let path = crate::engine::desired::skill_canonical(env, scope, name).join("SKILL.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (None, None);
    };
    let Some(front) = text
        .strip_prefix("---")
        .and_then(|rest| rest.find("\n---").map(|end| rest[..end].to_owned()))
    else {
        return (None, None);
    };
    let field = |key: &str| {
        front.lines().find_map(|line| {
            line.strip_prefix(key)
                .map(|v| v.trim().trim_matches('"').to_owned())
                .filter(|v| !v.is_empty())
        })
    };
    (field("source:"), field("repository:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A skill's frontmatter opts into upstream routing under either
    /// product-name token: catalogs rendered before the rename say
    /// `source: vstack` and must keep filing upstream.
    #[test]
    fn both_source_tokens_claim_ownership() {
        let lock = Lock::default();
        let owned = |source: Option<&str>| {
            is_kendex_owned(&lock, "s", None, source, None, DEFAULT_UPSTREAM)
        };
        assert!(owned(Some("kendex")));
        assert!(owned(Some("vstack")));
        assert!(!owned(Some("someone-else")));
        assert!(!owned(None));
    }
}
