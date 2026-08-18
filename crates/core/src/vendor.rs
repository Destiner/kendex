//! Content a coding tool ships with itself.
//!
//! A tool's own bundled plugins are not something a person installed, chose,
//! or can change: Codex writes OpenAI's plugin cache, Claude Code writes
//! Anthropic's. Auditing them tells the reader about a decision they never
//! made and cannot undo — the honest thing is to name who ships them and
//! leave them out of everything that asks the reader to act.
//!
//! Ownership is read off the marketplace a plugin names, because that is the
//! part a tool controls: `chrome@openai-bundled` is OpenAI's whatever
//! directory it landed in. A marketplace this table does not know is the
//! user's, never the vendor's — guessing the other way would silence a real
//! finding.

use crate::model::{HarnessId, ItemKind};

/// Who ships this content, when it is not the person running vstack.
pub fn vendor_of(kind: ItemKind, name: &str, harness: HarnessId) -> Option<&'static str> {
    if kind != ItemKind::Plugin {
        return None;
    }
    let (_, marketplace) = name.rsplit_once('@')?;
    match harness {
        HarnessId::Codex if marketplace.starts_with("openai-") => Some("OpenAI"),
        HarnessId::Claude if anthropic_marketplace(marketplace) => Some("Anthropic"),
        _ => None,
    }
}

fn anthropic_marketplace(marketplace: &str) -> bool {
    matches!(marketplace, "anthropic" | "anthropics" | "claude-code")
        || marketplace.starts_with("anthropics/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ownership_off_the_marketplace_a_plugin_names() {
        assert_eq!(
            vendor_of(ItemKind::Plugin, "chrome@openai-bundled", HarnessId::Codex),
            Some("OpenAI")
        );
        assert_eq!(
            vendor_of(
                ItemKind::Plugin,
                "docs@anthropics/claude-code",
                HarnessId::Claude
            ),
            Some("Anthropic")
        );
    }

    #[test]
    fn anything_else_belongs_to_whoever_installed_it() {
        // A marketplace nobody vouched for, a vendor's name under another
        // tool, and a kind that has no marketplace at all.
        assert_eq!(
            vendor_of(ItemKind::Plugin, "chrome@my-fork", HarnessId::Codex),
            None
        );
        assert_eq!(
            vendor_of(ItemKind::Plugin, "chrome@openai-bundled", HarnessId::Claude),
            None
        );
        assert_eq!(vendor_of(ItemKind::Skill, "deploy", HarnessId::Codex), None);
    }
}
