//! The git realities a search-table walk states rather than hydrates: a
//! skill's own frontmatter name, and submodules that would read as empty
//! skills if they were not named instead.

use crate::error::Result;
use crate::names;
use crate::source::plugin_registry::CatalogFinding;
use crate::source_read::SealedSource;

use super::SKILL_ROOTS;

pub(super) fn frontmatter_name(text: &str) -> Option<String> {
    let (yaml, _) = crate::frontmatter::split(text).ok()?;
    let parsed = crate::frontmatter::parse_tolerant(yaml).ok()?;
    parsed
        .map
        .get("name")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

/// Submodules are stated, not discovered: a pointer under a recognized root
/// would read as an empty skill, so it is named instead of hydrated.
pub(super) fn submodule_findings(
    sealed: &SealedSource,
    findings: &mut Vec<CatalogFinding>,
) -> Result<()> {
    let Some(text) = sealed.read_if_exists(&sealed.root().join(".gitmodules"))? else {
        return Ok(());
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "path" {
            continue;
        }
        let path = value.trim();
        if SKILL_ROOTS
            .iter()
            .any(|root| path == *root || path.starts_with(&format!("{root}/")))
        {
            findings.push(CatalogFinding::new(
                ".gitmodules",
                format!(
                    "`{}` is a submodule — its content is not fetched",
                    names::shown(path)
                ),
                "vendor the files in, or publish that repository as its own marketplace",
            ));
        }
    }
    Ok(())
}
