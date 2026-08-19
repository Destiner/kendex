//! Resolving a GitHub tree URL's `<ref>/<path>` remainder against the
//! mirror's real refs — branch names contain `/`, so the split point is
//! matched longest-first and an ambiguous split is refused, never guessed.

use crate::error::Result;

use super::refuse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Branch,
    Tag,
}

impl RefKind {
    pub fn word(self) -> &'static str {
        match self {
            RefKind::Branch => "branch",
            RefKind::Tag => "tag",
        }
    }
}

/// One branch or tag a mirror holds — what a tree URL's `<ref>/<path>`
/// resolves against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorRef {
    pub kind: RefKind,
    pub name: String,
}

impl MirrorRef {
    /// A full ref name as `for-each-ref` prints it. `None` for refs that
    /// are neither branches nor tags (remotes, notes).
    pub fn from_full(full: &str) -> Option<MirrorRef> {
        if let Some(name) = full.strip_prefix("refs/heads/") {
            return Some(MirrorRef {
                kind: RefKind::Branch,
                name: name.to_owned(),
            });
        }
        full.strip_prefix("refs/tags/").map(|name| MirrorRef {
            kind: RefKind::Tag,
            name: name.to_owned(),
        })
    }
}

/// A tree URL's `<ref>/<path>`, split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSplit {
    pub kind: RefKind,
    pub reference: String,
    /// The package path under the ref, when the URL carried one.
    pub path: Option<String>,
}

/// Split a tree URL's joined `<ref>/<path>` against the refs the mirror
/// actually holds, longest prefix first — branch names contain `/`, so no
/// string split can decide this. Exactly one ref may claim the prefix:
/// two split points both naming refs, or a branch and a tag sharing one
/// name, are refused naming every candidate — never guessed.
pub fn split_tree_ref(
    reference: &str,
    refs: &[MirrorRef],
    ref_and_path: &str,
) -> Result<TreeSplit> {
    let segments: Vec<&str> = ref_and_path.split('/').collect();
    let mut candidates = Vec::new();
    for take in (1..=segments.len()).rev() {
        let name = segments[..take].join("/");
        let path = (take < segments.len()).then(|| segments[take..].join("/"));
        for known in refs.iter().filter(|known| known.name == name) {
            candidates.push(TreeSplit {
                kind: known.kind,
                reference: name.clone(),
                path: path.clone(),
            });
        }
    }
    match candidates.as_slice() {
        [] => refuse(
            reference,
            format!("no branch or tag in the repository matches '{ref_and_path}'"),
        ),
        [only] => Ok(only.clone()),
        many => refuse(
            reference,
            format!(
                "ambiguous ref — could be {}",
                many.iter()
                    .map(|split| format!("{} '{}'", split.kind.word(), split.reference))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
        ),
    }
}
