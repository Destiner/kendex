//! Reading content the way a model reads it, not the way a byte comparison
//! does. Three passes: invisible characters come out, compatibility forms
//! collapse (NFKC), and letters that merely look Latin are folded to the
//! Latin letters they imitate. What the rules then match is the text a
//! reader sees, so `ignоre previous instructions` with a Cyrillic о is the
//! same string as the plain one.
//!
//! Nothing here is silent. Every change is counted per document and handed
//! to the `obfuscated-content` rule, because content that needs
//! deobfuscating to look clean has said something about itself.

use unicode_normalization::UnicodeNormalization;

use super::{AuditInput, Content, Doc, Prepared, Severity, TreeFile};

/// What deobfuscation had to do to one document. Only the two counts are
/// reportable: see `changed`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Normalization {
    pub location: String,
    /// Zero-width, bidi and joining characters removed. Variation
    /// selectors are not counted here — see `is_reportable`.
    pub invisible: usize,
    /// Letters folded to the Latin letters they imitate.
    pub homoglyphs: usize,
}

impl Normalization {
    /// Whether this is worth reporting.
    ///
    /// Deliberately not "did anything change". NFKC changes ordinary
    /// typography — an ellipsis, a non-breaking space, an `ﬁ` ligature —
    /// and emoji carry variation selectors by construction (`⚠️` is a
    /// warning sign plus U+FE0F). Both are stripped so that the other
    /// rules read a plain string, and neither says anything about intent.
    /// What is left — zero-width characters, bidirectional overrides,
    /// letters chosen to imitate other letters — has no typographic use.
    pub fn changed(&self) -> bool {
        self.invisible > 0 || self.homoglyphs > 0
    }
}

/// One line of a document, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub number: usize,
    pub text: String,
    /// ASCII-lowercased with whitespace flattened to spaces. Byte offsets
    /// match `text` exactly, so a match found here locates in the original.
    pub lower: String,
    /// Inside a code fence or a blockquote. Content here is documentation
    /// until proven otherwise, and its findings cost one severity less.
    pub fenced: bool,
    /// Byte ranges of inline `code` spans.
    spans: Vec<(usize, usize)>,
}

impl Line {
    /// Where `needle` sits in this line, allowing any run of whitespace
    /// where the needle has one space.
    pub fn find(&self, needle: &str) -> Option<usize> {
        find_phrase(&self.lower, needle)
    }

    pub fn has(&self, needle: &str) -> bool {
        self.find(needle).is_some()
    }

    /// True when this byte offset falls inside an inline code span — a flag
    /// written as `--force` is being described, not invoked.
    pub fn quoted_at(&self, at: usize) -> bool {
        self.spans
            .iter()
            .any(|(start, end)| at >= *start && at < *end)
    }

    /// This line read as description rather than instruction — what every
    /// line of a supporting file is.
    pub fn describing(self) -> Line {
        Line {
            fenced: true,
            ..self
        }
    }

    /// What a hit on `needle` weighs here: one severity less inside fenced
    /// or backticked content, full weight in live prose.
    pub fn weigh(&self, needle: &str, base: Severity) -> Severity {
        let quoted = self.find(needle).is_some_and(|at| self.quoted_at(at));
        match self.fenced || quoted {
            true => base.lowered(),
            false => base,
        }
    }
}

/// Substring search where one space in the needle matches any run of
/// whitespace in the haystack — `ignore  previous   instructions` is the
/// same phrase as the single-spaced one.
pub fn find_phrase(hay: &str, needle: &str) -> Option<usize> {
    let hay = hay.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return None;
    }
    'start: for start in 0..hay.len() {
        let mut h = start;
        let mut n = 0;
        while n < needle.len() {
            if needle[n] == b' ' {
                if h >= hay.len() || hay[h] != b' ' {
                    continue 'start;
                }
                while h < hay.len() && hay[h] == b' ' {
                    h += 1;
                }
                n += 1;
                continue;
            }
            if h >= hay.len() || hay[h] != needle[n] {
                continue 'start;
            }
            h += 1;
            n += 1;
        }
        return Some(start);
    }
    None
}

/// Deobfuscate every text this input carries and split it into lines.
pub fn prepare(input: AuditInput) -> Prepared {
    let mut normalized = Vec::new();
    let mut docs = Vec::new();
    let mut clean = |location: String, text: &str| -> String {
        let (out, report) = deobfuscate(&location, text);
        if report.changed() {
            normalized.push(report);
        }
        out
    };
    let content = match input.content {
        Content::Document { text } => {
            let text = clean(input.location.clone(), &text);
            docs.push(Doc {
                location: input.location.clone(),
                lines: lines(&text),
            });
            Content::Document { text }
        }
        Content::SkillTree { files } => Content::SkillTree {
            files: tree_docs(&input.location, files, &mut clean, &mut docs),
        },
        Content::Hook {
            event,
            matcher,
            command,
            script,
        } => hook_content(
            &input.location,
            event,
            matcher,
            command,
            script,
            &mut clean,
            &mut docs,
        ),
        Content::Mcp(entry) => Content::Mcp(entry),
        Content::Unread { why } => Content::Unread { why },
        Content::Plugin(sources) => Content::Plugin(super::PluginSources {
            scripts: tree_docs(&input.location, sources.scripts, &mut clean, &mut docs),
            ..sources
        }),
    };
    Prepared {
        input: AuditInput { content, ..input },
        docs,
        normalized,
    }
}

fn tree_docs(
    root: &str,
    files: Vec<TreeFile>,
    clean: &mut impl FnMut(String, &str) -> String,
    docs: &mut Vec<Doc>,
) -> Vec<TreeFile> {
    files
        .into_iter()
        .map(|file| {
            let Some(text) = file.text else {
                return TreeFile { text: None, ..file };
            };
            let location = format!("{root}/{}", file.path.display());
            let supporting = is_supporting(&file.path);
            let text = clean(location.clone(), &text);
            docs.push(Doc {
                lines: match supporting {
                    true => lines(&text).into_iter().map(Line::describing).collect(),
                    false => lines(&text),
                },
                location,
            });
            TreeFile {
                text: Some(text),
                ..file
            }
        })
        .collect()
}

/// A file that comes along with a skill rather than being what a harness
/// loads. Its findings weigh one severity less, for the same reason a
/// fenced example does: a test asserting that a command line is passed
/// through is describing that command line, not issuing it.
///
/// This was settled by a real catalog. The vstack `orch` skill ships tests
/// that assert `--dangerously-skip-permissions` reaches the launcher, and
/// the `review-gate` skill has a test that base64-encodes a fixture. Both
/// are exactly what those rules look for, and neither is the skill telling
/// a model to do anything. A key in one of these files still counts in
/// full, because `plaintext-secrets` never downgrades.
fn is_supporting(path: &std::path::Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some("tests" | "test" | "__tests__" | "fixtures" | "testdata")
        )
    })
}

fn hook_content(
    root: &str,
    event: String,
    matcher: Option<String>,
    command: String,
    script: Option<String>,
    clean: &mut impl FnMut(String, &str) -> String,
    docs: &mut Vec<Doc>,
) -> Content {
    let command = clean(format!("{root} (command)"), &command);
    docs.push(Doc {
        location: format!("{root} (command)"),
        lines: lines(&command),
    });
    let script = script.map(|body| {
        let body = clean(root.to_owned(), &body);
        docs.push(Doc {
            location: root.to_owned(),
            lines: lines(&body),
        });
        body
    });
    Content::Hook {
        event,
        matcher,
        command,
        script,
    }
}

/// Invisible characters out, NFKC, then homoglyphs folded — in that order,
/// so a fullwidth letter becomes ASCII before the confusable table sees it.
pub fn deobfuscate(location: &str, text: &str) -> (String, Normalization) {
    let mut report = Normalization {
        location: location.to_owned(),
        ..Normalization::default()
    };
    let stripped: String = text
        .chars()
        .filter(|c| {
            let invisible = is_invisible(*c);
            report.invisible += usize::from(invisible && is_reportable(*c));
            !invisible
        })
        .collect();
    let out: String = stripped
        .nfkc()
        .collect::<String>()
        .chars()
        .map(|c| match fold_homoglyph(c) {
            Some(latin) => {
                report.homoglyphs += 1;
                latin
            }
            None => c,
        })
        .collect();
    (out, report)
}

/// Characters that occupy no space on screen: zero-width joiners and
/// spaces, bidirectional overrides, word joiners, variation selectors and
/// the byte-order mark. All of them come out before the rules read a line.
fn is_invisible(c: char) -> bool {
    matches!(c as u32,
        0x00AD | 0x180E | 0xFEFF
        | 0x200B..=0x200F
        | 0x202A..=0x202E
        | 0x2060..=0x2064
        | 0x2066..=0x2069
        | 0xFE00..=0xFE0F
        | 0xE0100..=0xE01EF)
}

/// Which of those are worth reporting. Variation selectors are how every
/// emoji is spelled — `⚠️` is U+26A0 followed by U+FE0F — so counting them
/// would flag every shell script that prints a warning sign.
fn is_reportable(c: char) -> bool {
    !matches!(c as u32, 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
}

/// Cyrillic and Greek letters that are drawn as Latin ones. The table is
/// deliberately narrow: only characters whose confusion turns one readable
/// English word into another.
fn fold_homoglyph(c: char) -> Option<char> {
    const CYRILLIC: &str = "аеорсухіјѕАВЕКМНОРСТУХ";
    const CYRILLIC_LATIN: &str = "aeopcyxijsABEKMHOPCTYX";
    const GREEK: &str = "ΑΒΕΖΗΙΚΜΝΟΡΤΥΧοναρ";
    const GREEK_LATIN: &str = "ABEZHIKMNOPTYXovap";
    let lookup = |from: &str, to: &str| {
        from.chars()
            .position(|candidate| candidate == c)
            .and_then(|index| to.chars().nth(index))
    };
    lookup(CYRILLIC, CYRILLIC_LATIN).or_else(|| lookup(GREEK, GREEK_LATIN))
}

/// Split into lines, marking which sit inside a code fence or a blockquote
/// and where each line's inline code spans are.
pub fn lines(text: &str) -> Vec<Line> {
    let mut fence: Option<String> = None;
    text.lines()
        .enumerate()
        .map(|(index, raw)| {
            let trimmed = raw.trim_start();
            let delimiter = fence_delimiter(trimmed);
            let inside = fence.is_some();
            let is_delimiter = delimiter.is_some();
            match (&fence, delimiter) {
                (Some(open), Some(found)) if found.starts_with(open.as_str()) => fence = None,
                (None, Some(found)) => fence = Some(found),
                _ => {}
            }
            Line {
                number: index + 1,
                lower: flatten(raw),
                fenced: inside || is_delimiter || trimmed.starts_with('>'),
                spans: code_spans(raw),
                text: raw.to_owned(),
            }
        })
        .collect()
}

/// The backtick or tilde run that opens or closes a fence, when the line is
/// one. Three or more of the same character, per CommonMark.
fn fence_delimiter(trimmed: &str) -> Option<String> {
    let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run: String = trimmed.chars().take_while(|c| *c == marker).collect();
    (run.chars().count() >= 3).then_some(run)
}

/// Byte ranges covered by inline `code` spans, backticks included.
fn code_spans(raw: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut open: Option<usize> = None;
    for (offset, c) in raw.char_indices() {
        if c != '`' {
            continue;
        }
        match open.take() {
            Some(start) => spans.push((start, offset + 1)),
            None => open = Some(offset),
        }
    }
    spans
}

/// ASCII-lowercase with every whitespace byte turned into a space. Both
/// operations are byte-for-byte, so offsets still index the original line.
fn flatten(raw: &str) -> String {
    raw.chars()
        .map(|c| match c.is_ascii() {
            true if c.is_ascii_whitespace() => ' ',
            true => c.to_ascii_lowercase(),
            false => c,
        })
        .collect()
}
