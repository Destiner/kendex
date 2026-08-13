//! What an item name may be, and when two names are the same name.
//!
//! Every declared name becomes a file or directory somewhere, so a name that
//! cannot be one is refused where it is written down rather than where it
//! fails. Marketplace-shaped catalogs add a second rule: a name may carry
//! one `<plugin>/<leaf>` segment pair, and nothing more.

/// Room for the separator a namespaced name expands to, the `.disabled`
/// parking suffix, and Copilot's `.agent.md` — inside the 255-byte
/// component limit every filesystem vstack installs to shares.
const MAX_SEGMENT: usize = 100;

/// Names Windows keeps for devices. A file with one of these stems is not
/// created, it is written to the device, whatever extension it carries.
const DEVICE_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Why this one path segment cannot be a file or directory name.
pub fn segment_problem(segment: &str) -> Option<String> {
    if segment.is_empty() {
        return Some("a name cannot be empty".to_owned());
    }
    if segment == "." || segment == ".." {
        return Some(format!("`{segment}` names a directory, not an item"));
    }
    if segment.starts_with('-') {
        return Some(format!(
            "`{segment}` starts with `-`, which reads as a flag"
        ));
    }
    if let Some(bad) = segment
        .chars()
        .find(|c| c.is_control() || "\\:*?\"<>|".contains(*c))
    {
        return Some(format!("`{segment}` holds `{bad}`, which no filename may"));
    }
    if segment.ends_with('.') || segment.ends_with(' ') {
        return Some(format!(
            "`{segment}` ends in a dot or a space, which Windows silently drops"
        ));
    }
    let stem = segment.split('.').next().unwrap_or(segment);
    if DEVICE_NAMES.contains(&stem.to_ascii_lowercase().as_str()) {
        return Some(format!("`{segment}` is a reserved device name on Windows"));
    }
    if segment.len() > MAX_SEGMENT {
        return Some(format!(
            "`{segment}` is {} bytes and a name may be {MAX_SEGMENT}",
            segment.len()
        ));
    }
    None
}

/// Why this item name cannot be installed. A name from a marketplace-shaped
/// catalog carries its plugin — `<plugin>/<leaf>` — and that is the only
/// `/` any name may hold.
pub fn item_problem(name: &str) -> Option<String> {
    let mut segments = name.split('/');
    let first = segments.next().unwrap_or_default();
    if let Some(problem) = segment_problem(first) {
        return Some(problem);
    }
    // No second segment is a plain name, which is legal.
    let leaf = segments.next()?;
    if segments.next().is_some() {
        return Some(format!(
            "`{name}` has more than one `/` — a name is either a plain name or `<plugin>/<item>`"
        ));
    }
    segment_problem(leaf)
}

/// The plugin and item halves of a namespaced name, or `None` for a plain
/// one. Callers that resolve paths use this rather than splitting again:
/// the split and the legality rule above must never disagree.
pub fn split(name: &str) -> Option<(&str, &str)> {
    let (plugin, leaf) = name.split_once('/')?;
    (item_problem(name).is_none()).then_some((plugin, leaf))
}

/// The spelling two names collide under. Case is folded because macOS and
/// Windows hand the same file to both spellings, and trailing dots because
/// Windows drops them — on those systems the second install silently
/// overwrites the first.
pub fn fold(name: &str) -> String {
    name.to_lowercase()
        .split('/')
        .map(|segment| segment.trim_end_matches(['.', ' ']).to_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_and_one_plugin_segment_are_legal() {
        assert!(item_problem("gh").is_none());
        assert!(item_problem("data-science/eda").is_none());
        assert_eq!(split("data-science/eda"), Some(("data-science", "eda")));
        assert_eq!(split("gh"), None);
    }

    #[test]
    fn path_hostile_shapes_are_named_with_the_reason() {
        for name in [
            "..",
            "../etc",
            "a/../b",
            "a/b/c",
            "",
            "a/",
            "/b",
            "nul",
            "com1.md",
            "trailing.",
            "-flag",
            "back\\slash",
        ] {
            assert!(item_problem(name).is_some(), "{name} should be refused");
        }
        assert!(item_problem(&"x".repeat(MAX_SEGMENT + 1)).is_some());
        assert!(item_problem(&"x".repeat(MAX_SEGMENT)).is_none());
    }

    #[test]
    fn folding_catches_the_collisions_a_filesystem_would_make() {
        assert_eq!(fold("Data-Science/EDA"), "data-science/eda");
        assert_eq!(fold("gh."), "gh");
        assert_ne!(fold("a/b"), fold("a-b"));
    }
}
