//! What an item is *for*, as a small closed vocabulary.
//!
//! A kind already says what a thing is (a skill, an agent, a hook); a tag
//! says what job it helps with. One axis only: mixing "what it does" with
//! "what it is about" and "what shape it takes" gives a filter that answers
//! three questions at once and none of them well.
//!
//! The vocabulary is closed on purpose. Free-form tags drift — `test`,
//! `tests`, `testing` and `unit-test` become four buckets holding one idea,
//! and a filter built on them is a guess. An unknown tag is reported as a
//! warning naming the ones that exist, so the author finds out rather than
//! silently getting no tag at all.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use specta::Type;

/// The job an item helps with.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum Tag {
    /// Reading code and saying what is wrong with it.
    Review,
    /// Writing, running, or reasoning about tests.
    Testing,
    /// Documentation, references, and explaining things in prose.
    Docs,
    /// Finding things out — sources, prior art, evidence.
    Research,
    /// Deciding what to build and in what order.
    Planning,
    /// Reshaping code that already works.
    Refactoring,
    /// Working out why something is broken.
    Debugging,
    /// Safety, secrets, permissions, and untrusted input.
    Security,
    /// Speed, memory, and the cost of running something.
    Performance,
    /// Version control — branches, history, worktrees, pull requests.
    Git,
    /// Compiling, packaging, versioning, and shipping a release.
    Release,
    /// Databases, schemas, migrations, and analysis.
    Data,
    /// Interfaces: layout, styling, design systems, accessibility.
    Ui,
    /// Talking to other services — APIs, webhooks, third-party tools.
    Integration,
    /// Driving other tools and agents, and the workflows that chain them.
    Automation,
}

/// Every tag, in the order they are offered to a reader.
pub const ALL_TAGS: &[Tag] = &[
    Tag::Review,
    Tag::Testing,
    Tag::Debugging,
    Tag::Refactoring,
    Tag::Planning,
    Tag::Research,
    Tag::Docs,
    Tag::Security,
    Tag::Performance,
    Tag::Git,
    Tag::Release,
    Tag::Data,
    Tag::Ui,
    Tag::Integration,
    Tag::Automation,
];

impl Tag {
    /// The wire and authoring spelling — what goes in a file's frontmatter.
    pub fn name(self) -> &'static str {
        match self {
            Tag::Review => "review",
            Tag::Testing => "testing",
            Tag::Docs => "docs",
            Tag::Research => "research",
            Tag::Planning => "planning",
            Tag::Refactoring => "refactoring",
            Tag::Debugging => "debugging",
            Tag::Security => "security",
            Tag::Performance => "performance",
            Tag::Git => "git",
            Tag::Release => "release",
            Tag::Data => "data",
            Tag::Ui => "ui",
            Tag::Integration => "integration",
            Tag::Automation => "automation",
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Tag {
    type Err = ();

    /// Case and surrounding space are the author's business, not ours; the
    /// spelling itself has to match, so a near-miss is reported rather than
    /// guessed at.
    fn from_str(text: &str) -> Result<Tag, ()> {
        let wanted = text.trim().to_ascii_lowercase();
        ALL_TAGS
            .iter()
            .copied()
            .find(|tag| tag.name() == wanted)
            .ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tag_round_trips_through_its_written_form() {
        for tag in ALL_TAGS {
            assert_eq!(tag.name().parse::<Tag>(), Ok(*tag));
        }
    }

    /// `ALL_TAGS` is what `FromStr` searches and what the vocabulary order
    /// comes from, so a variant missing from it is a tag no file can use and
    /// nothing else would catch. The match is exhaustive: adding a variant
    /// stops this compiling until it is listed here and there.
    #[test]
    fn the_vocabulary_lists_every_tag() {
        fn accounted_for(tag: Tag) -> bool {
            match tag {
                Tag::Review
                | Tag::Testing
                | Tag::Docs
                | Tag::Research
                | Tag::Planning
                | Tag::Refactoring
                | Tag::Debugging
                | Tag::Security
                | Tag::Performance
                | Tag::Git
                | Tag::Release
                | Tag::Data
                | Tag::Ui
                | Tag::Integration
                | Tag::Automation => true,
            }
        }
        assert_eq!(ALL_TAGS.len(), 15);
        assert!(ALL_TAGS.iter().copied().all(accounted_for));
        let unique: std::collections::BTreeSet<Tag> = ALL_TAGS.iter().copied().collect();
        assert_eq!(unique.len(), ALL_TAGS.len());
    }

    /// Two spellings of the same tag exist: serde's, which the UI filter
    /// sends, and `name()`, which frontmatter is matched against. They have
    /// to be the same word or a tag parses from a file and never matches the
    /// filter.
    #[test]
    fn the_written_form_and_the_wire_form_agree() {
        for tag in ALL_TAGS {
            let wire = serde_json::to_string(tag).unwrap();
            assert_eq!(wire, format!("\"{}\"", tag.name()));
        }
    }
}
