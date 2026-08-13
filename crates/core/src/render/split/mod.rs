//! Body-size cap for a rendered skill. Harnesses that read only the first N
//! bytes of SKILL.md truncate silently, so the end of a long skill is simply
//! lost at load. Splitting it ourselves — head plus a pointer into
//! `references/` — keeps every byte reachable and makes the loss visible as a
//! warning instead of as missing behavior.

use std::path::{Path, PathBuf};

use super::RenderWarning;
use super::fences::fenced_ranges;
use super::skill::{INSTRUCTIONS_END, INSTRUCTIONS_START};

const SKILL_FILE: &str = "SKILL.md";
const PROVENANCE: &str = "<!-- continued from SKILL.md -->\n";
const FIX: &str = "shorten SKILL.md or move detail into references/ yourself";

pub struct SplitOutcome {
    /// The adjusted tree (SKILL.md possibly shortened, an overflow file
    /// possibly added). Unchanged when the body already fits.
    pub files: Vec<(PathBuf, Vec<u8>)>,
    pub warnings: Vec<RenderWarning>,
    /// Set when the cap cannot be honored at all (a single fenced block
    /// larger than the cap): the caller refuses the install for that
    /// surface instead of cutting mid-fence.
    pub refusal: Option<String>,
}

impl SplitOutcome {
    fn unchanged(files: Vec<(PathBuf, Vec<u8>)>) -> SplitOutcome {
        SplitOutcome {
            files,
            warnings: Vec::new(),
            refusal: None,
        }
    }

    fn refused(files: Vec<(PathBuf, Vec<u8>)>, reason: String) -> SplitOutcome {
        SplitOutcome {
            files,
            warnings: Vec::new(),
            refusal: Some(reason),
        }
    }
}

/// Enforce `max_bytes` on the tree's SKILL.md, moving whatever does not fit
/// into `references/`. Frontmatter and the injected project-instructions
/// block always stay in the head: instructions the project added are
/// authoritative and must be read before anything they qualify.
pub fn enforce_body_cap(mut files: Vec<(PathBuf, Vec<u8>)>, max_bytes: usize) -> SplitOutcome {
    let Some(index) = files
        .iter()
        .position(|(path, _)| path == Path::new(SKILL_FILE))
    else {
        return SplitOutcome::unchanged(files);
    };
    if files[index].1.len() <= max_bytes {
        return SplitOutcome::unchanged(files);
    }
    let Ok(text) = std::str::from_utf8(&files[index].1).map(str::to_owned) else {
        let reason = format!("{SKILL_FILE} is not valid UTF-8, so it cannot be cut safely");
        return SplitOutcome::refused(files, reason);
    };

    let front = crate::frontmatter::split(&text).map_or(0, |(_, body)| text.len() - body.len());
    let body = &text[front..];
    let protected = protected_range(body);
    let block = protected.1 - protected.0;
    let overflow = overflow_path(&files);
    let note = format!(
        "\n> Continued in {} — read it for the remaining sections.\n",
        overflow.display()
    );
    // What the head may spend on body bytes: everything up to the cut, plus
    // the protected block. A cap too small to hold the block on its own
    // cannot be met at all.
    let Some(spare) = max_bytes.checked_sub(front + note.len() + block) else {
        let reason = format!(
            "{SKILL_FILE} cannot meet the {max_bytes}-byte cap: its frontmatter and project instructions alone are {} bytes",
            front + block
        );
        return SplitOutcome::refused(files, reason);
    };
    let budget = spare + block;

    // Ranges no cut may land inside: code blocks, and the protected block,
    // whose own `## ` heading would otherwise look like a split point.
    let mut forbidden = fenced_ranges(body);
    if block > 0 {
        forbidden.push(protected);
    }
    // Past the block the budget covers it already; before the block the cut
    // has to leave room for it.
    let ceiling = match budget >= protected.1 {
        true => budget.min(body.len()),
        false => spare.min(protected.0),
    };
    let cut = headings(body, &forbidden)
        .into_iter()
        .rev()
        .find(|offset| *offset <= ceiling)
        .unwrap_or_else(|| hard_cut(body, &forbidden, ceiling));

    if body[..cut].trim().is_empty() && inside(&forbidden, ceiling) {
        let reason = format!(
            "{SKILL_FILE} is {} bytes, over the {max_bytes}-byte cap, and the text spanning the limit is one fenced code block — cutting there would break the fence",
            text.len()
        );
        return SplitOutcome::refused(files, reason);
    }

    let (kept, moved) = match cut <= protected.0 {
        true => (
            format!("{}{}", &body[..cut], &body[protected.0..protected.1]),
            format!("{}{}", &body[cut..protected.0], &body[protected.1..]),
        ),
        false => (body[..cut].to_owned(), body[cut..].to_owned()),
    };
    let head = format!("{}{kept}{note}", &text[..front]);
    debug_assert!(head.len() <= max_bytes);

    let warning = RenderWarning::with_fix(
        format!(
            "{SKILL_FILE} was {} bytes, over the {max_bytes}-byte cap — {} bytes moved to {}, leaving {} bytes",
            text.len(),
            moved.len(),
            overflow.display(),
            head.len()
        ),
        FIX,
    );
    files[index].1 = head.into_bytes();
    files.push((overflow, format!("{PROVENANCE}{moved}").into_bytes()));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    SplitOutcome {
        files,
        warnings: vec![warning],
        refusal: None,
    }
}

/// The injected project-instructions block, as a body byte range. An
/// unterminated marker is user damage — as in `inject_instructions`, we take
/// the file as it stands rather than guessing where the block ends.
fn protected_range(body: &str) -> (usize, usize) {
    let absent = (body.len(), body.len());
    let Some(start) = body.find(INSTRUCTIONS_START) else {
        return absent;
    };
    let Some(end) = body[start..].find(INSTRUCTIONS_END) else {
        return absent;
    };
    let mut end = start + end + INSTRUCTIONS_END.len();
    if body.as_bytes().get(end) == Some(&b'\n') {
        end += 1;
    }
    (start, end)
}

/// The first free overflow name. Never overwrite a file the skill author
/// wrote: a `references/details.md` of their own keeps its content.
fn overflow_path(files: &[(PathBuf, Vec<u8>)]) -> PathBuf {
    let taken = |path: &Path| files.iter().any(|(existing, _)| existing == path);
    let mut path = Path::new("references").join("details.md");
    let mut attempt = 1;
    while taken(&path) {
        let name = match attempt {
            1 => "details_overflow.md".to_owned(),
            n => format!("details_overflow-{n}.md"),
        };
        path = Path::new("references").join(name);
        attempt += 1;
    }
    path
}

/// Offsets of the section headings a cut can land on: any heading from `##`
/// to `######` that starts a line outside every forbidden range. Deeper
/// headings count — a skill whose sections are all `###` would otherwise
/// keep a title and nothing else, and send the reader a pointer instead of a
/// skill.
fn headings(body: &str, forbidden: &[(usize, usize)]) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        if is_heading(line) && !inside(forbidden, offset) {
            offsets.push(offset);
        }
        offset += line.len();
    }
    offsets
}

fn is_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    (2..=6).contains(&hashes) && line[hashes..].starts_with(' ')
}

/// Cut at the last line boundary under the ceiling, or — when the ceiling
/// falls inside the body's first line — at the last character boundary under
/// it, so a multibyte character is never severed.
fn hard_cut(body: &str, forbidden: &[(usize, usize)], ceiling: usize) -> usize {
    let mut cut = 0;
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        offset += line.len();
        if offset > ceiling {
            break;
        }
        if !inside(forbidden, offset) {
            cut = offset;
        }
    }
    if cut > 0 || inside(forbidden, ceiling) {
        return cut;
    }
    let mut cut = ceiling;
    while !body.is_char_boundary(cut) {
        cut -= 1;
    }
    cut
}

/// Strictly inside: a range's own start and end are legal cut points, which
/// is how a whole fenced block moves to the overflow file intact.
fn inside(ranges: &[(usize, usize)], offset: usize) -> bool {
    ranges
        .iter()
        .any(|(start, end)| offset > *start && offset < *end)
}

#[cfg(test)]
mod tests;
