//! What a catalog states about itself in `[marketplace]` — read-only,
//! untrusted: every string is control-char-safe and capped before it is
//! stored, because this text travels into terminals, the app, and the
//! community directory.

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct MarketplaceMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub tags: Vec<String>,
}

/// A short field: a name, an author, a license, a homepage.
const MAX_META_TEXT: usize = 200;
/// A description says more, but not a README's worth.
const MAX_META_DESCRIPTION: usize = 500;
const MAX_META_TAGS: usize = 10;
const MAX_META_TAG: usize = 40;

/// Catalog-authored text made safe to show: control characters escaped,
/// length capped in characters.
pub fn safe_text(text: &str, max: usize) -> String {
    crate::names::shown(text.trim()).chars().take(max).collect()
}

impl MarketplaceMeta {
    pub(super) fn capped(self) -> MarketplaceMeta {
        let field = |text: Option<String>, max| {
            text.map(|text| safe_text(&text, max))
                .filter(|text: &String| !text.is_empty())
        };
        MarketplaceMeta {
            name: field(self.name, MAX_META_TEXT),
            description: field(self.description, MAX_META_DESCRIPTION),
            author: field(self.author, MAX_META_TEXT),
            license: field(self.license, MAX_META_TEXT),
            homepage: field(self.homepage, MAX_META_TEXT),
            tags: self
                .tags
                .into_iter()
                .take(MAX_META_TAGS)
                .map(|tag| safe_text(&tag, MAX_META_TAG))
                .filter(|tag| !tag.is_empty())
                .collect(),
        }
    }
}
