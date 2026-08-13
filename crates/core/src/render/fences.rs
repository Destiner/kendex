//! Where the fenced code blocks are. Two passes must leave them alone: the
//! body-cap splitter, which may not cut inside one, and the prose rewrite,
//! which may not reword one. They read the same scanner, so a block one of
//! them sees is a block to the other.

/// A fence line: any leading whitespace, then three or more backticks or
/// tildes. `bare` — nothing but whitespace after the run — is what makes a
/// line eligible to close a fence rather than open one.
///
/// Indent is not a limit. Markdown allows three spaces at the top level, but
/// a block nested inside a list item starts four in, and that is the common
/// shape in a real skill: a scanner that stops at three reads the block as
/// prose and cuts or rewrites straight through it.
pub fn fence_marker(line: &str) -> Option<(char, usize, bool)> {
    let rest = line.trim_start();
    let marker = rest.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let run = rest.chars().take_while(|c| *c == marker).count();
    (run >= 3).then(|| (marker, run, rest[run..].trim().is_empty()))
}

/// Byte ranges covered by fenced code blocks. A fence closes only on a run of
/// at least as many of the same character, so a four-backtick fence survives
/// the three-backtick runs it quotes; an unclosed fence runs to the end.
pub fn fenced_ranges(body: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut open: Option<(char, usize, usize)> = None;
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        match (fence_marker(line), open) {
            (Some((marker, run, bare)), Some((wanted, len, start)))
                if marker == wanted && run >= len && bare =>
            {
                ranges.push((start, offset + line.len()));
                open = None;
            }
            (Some((marker, run, _)), None) => open = Some((marker, run, offset)),
            _ => {}
        }
        offset += line.len();
    }
    if let Some((_, _, start)) = open {
        ranges.push((start, body.len()));
    }
    ranges
}
