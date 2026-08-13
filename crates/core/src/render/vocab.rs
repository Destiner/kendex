//! What each harness calls the tools an agent talks about. Agent bodies are
//! authored in Claude's vocabulary — "the Read tool" — and every other
//! harness reads that as a name for something it does not have. This module
//! owns all three translations: the manifest-name tables the renderers use
//! for their permission fields, and the conservative prose rewrite that lets
//! a body say the same thing in the reader's own words.

use super::RenderWarning;
use crate::model::HarnessId;

/// v1's alias table: manifests write generic lowercase tool names, Claude
/// matches exact PascalCase — an unmapped name silently fails to deny.
pub fn claude_tool_name(tool: &str) -> String {
    match normalize(tool).as_str() {
        "read" => "Read".into(),
        "grep" => "Grep".into(),
        "glob" | "find" => "Glob".into(),
        "ls" | "list" => "LS".into(),
        "bash" => "Bash".into(),
        "edit" => "Edit".into(),
        "multiedit" => "MultiEdit".into(),
        "write" => "Write".into(),
        "webfetch" => "WebFetch".into(),
        "websearch" => "WebSearch".into(),
        "todowrite" => "TodoWrite".into(),
        "todoread" => "TodoRead".into(),
        "task" | "agent" | "subagent" | "spawnagent" | "spawnagentsoncsv" => "Agent".into(),
        "question" | "askuserquestion" => "AskUserQuestion".into(),
        "notebookread" => "NotebookRead".into(),
        "notebookedit" => "NotebookEdit".into(),
        _ => tool.trim().to_owned(),
    }
}

/// OpenCode gates tools by permission key, not tool name. `None` is the
/// empty name — nothing to gate; an unknown name passes through so an MCP
/// tool can still be denied by its own id.
pub fn opencode_permission(tool: &str) -> Option<String> {
    let permission = match normalize(tool).as_str() {
        "read" => "read",
        "edit" | "write" | "patch" | "applypatch" | "multiedit" | "notebookedit" => "edit",
        "glob" | "find" | "ls" | "list" => "glob",
        "grep" => "grep",
        "bash" | "shell" => "bash",
        "task" | "agent" | "subagent" | "spawnagent" | "spawnagentsoncsv" => "task",
        "skill" => "skill",
        "lsp" => "lsp",
        "question" => "question",
        "webfetch" | "websearch" | "web" | "webresearch" | "webanswer" | "codesearch" => "webfetch",
        "" => return None,
        _ => return Some(tool.trim().to_owned()),
    };
    Some(permission.to_owned())
}

/// Claude's own spelling for every tool a body can name. Recognition in
/// prose is exact against this list: `read` in a sentence is the verb,
/// `Read` is the tool.
const CLAUDE_TOOLS: [&str; 16] = [
    "Read",
    "Grep",
    "Glob",
    "LS",
    "Bash",
    "Edit",
    "MultiEdit",
    "Write",
    "WebFetch",
    "WebSearch",
    "TodoWrite",
    "TodoRead",
    "Agent",
    "AskUserQuestion",
    "NotebookRead",
    "NotebookEdit",
];

/// Skill pointers name generated files; a line that carries one is a path
/// reference, not prose, and stays byte-for-byte.
const SKILL_POINTER: &str = "SKILL.md";

/// How a harness says a tool: a name that slots into the same sentence, or
/// — for Codex, whose docs name actions rather than tools — a phrase that
/// stands in for the whole reference.
enum Word {
    Name(&'static str),
    Phrase(&'static str),
}

/// The vocabulary each harness has an official word for. A tool missing
/// from a harness's column is left as authored rather than guessed at.
fn word(tool: &str, harness: HarnessId) -> Option<Word> {
    let tool = normalize(tool);
    match harness {
        // Bodies are already written in Claude's words.
        HarnessId::Claude => None,
        HarnessId::Codex => Some(Word::Phrase(match tool.as_str() {
            "read" => "open the file",
            "grep" => "search",
            "glob" | "ls" => "list files",
            "bash" => "run a shell command",
            "edit" | "multiedit" | "write" => "edit the file",
            "webfetch" | "websearch" => "fetch the page",
            _ => return None,
        })),
        HarnessId::Opencode | HarnessId::Cursor | HarnessId::Pi => {
            Some(Word::Name(match tool.as_str() {
                "read" => "read",
                "grep" => "grep",
                "glob" => "glob",
                "ls" => "list",
                "bash" => "bash",
                "edit" | "multiedit" => "edit",
                "write" => "write",
                "webfetch" | "websearch" => "webfetch",
                _ => return None,
            }))
        }
    }
}

fn normalize(tool: &str) -> String {
    tool.trim().to_lowercase().replace(['_', '-'], "")
}

/// Say the body's tool references in `harness`'s vocabulary. Only two
/// shapes are touched — `the Read tool` and `` `Read` tool `` — because
/// only they can mean a tool and nothing else. Samples the agent is meant
/// to copy (code fences, inline literals), links, and generated skill
/// paths keep every byte, and a name this module does not know is reported
/// rather than guessed at.
///
/// Renderers pass the agent's own body and nothing else. Launch and
/// additional instructions are the project's words about this project;
/// rewriting them would put words in the author's mouth, and the author is
/// there to change them.
pub fn rewrite_prose(body: &str, harness: HarnessId) -> (String, Vec<RenderWarning>) {
    if harness == HarnessId::Claude {
        return (body.to_owned(), Vec::new());
    }
    let mut out = String::with_capacity(body.len());
    let mut reworded: Vec<String> = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in body.split_inclusive('\n') {
        match (fence_marker(line), fence) {
            (Some((marker, run, bare)), Some((open, len)))
                if marker == open && run >= len && bare =>
            {
                fence = None;
            }
            (Some((marker, run, _)), None) => fence = Some((marker, run)),
            _ if fence.is_none() && !line.contains(SKILL_POINTER) => {
                out.push_str(&rewrite_line(line, harness, &mut reworded, &mut kept));
                continue;
            }
            _ => {}
        }
        out.push_str(line);
    }

    let mut warnings = Vec::new();
    if !reworded.is_empty() {
        warnings.push(RenderWarning::new(format!(
            "tool references reworded for {}: {}",
            harness.display_name(),
            reworded.join(", ")
        )));
    }
    warnings.extend(kept.iter().map(|tool| {
        RenderWarning::new(format!(
            "`{tool}` is not a {} tool name — the reference passes through as written",
            harness.display_name()
        ))
    }));
    (out, warnings)
}

fn rewrite_line(
    line: &str,
    harness: HarnessId,
    reworded: &mut Vec<String>,
    kept: &mut Vec<String>,
) -> String {
    let spans = code_spans(line);
    let links = link_ranges(line);
    let mut out = String::with_capacity(line.len());
    let mut copied = 0;
    for (at, _) in line.match_indices("tool") {
        let Some(reference) = reference_before(line, at) else {
            continue;
        };
        let (from, to) = reference.name;
        let name = &line[from..to];
        // A link is a target, and a code span holding more than the name
        // itself is a sample to copy — neither is prose about a tool.
        let quoted_reference =
            |(open, close): &(usize, usize)| line[*open..*close].trim_matches('`').trim() == name;
        if reference.start < copied
            || links
                .iter()
                .any(|(open, close)| from >= *open && from < *close)
            || spans
                .iter()
                .any(|span| from > span.0 && to < span.1 && !quoted_reference(span))
        {
            continue;
        }
        let (from, to, said) = match (CLAUDE_TOOLS.contains(&name), word(name, harness)) {
            (true, Some(Word::Name(said))) => (from, to, said.to_owned()),
            (true, Some(Word::Phrase(said))) => match reference.capitalized {
                true => (reference.start, at + 4, capitalize(said)),
                false => (reference.start, at + 4, said.to_owned()),
            },
            // A tool this harness has no word for, and any name that is not
            // ours to translate — an MCP id, a plugin's own tool.
            (true, None) | (false, _) if tool_shaped(name) => {
                remember(kept, name);
                continue;
            }
            _ => continue,
        };
        remember(reworded, name);
        out.push_str(&line[copied..from]);
        out.push_str(&said);
        copied = to;
    }
    out.push_str(&line[copied..]);
    out
}

/// The tool reference ending at the word `tool` that starts at `at`: where
/// the whole reference starts (article and backticks included), the name's
/// own range, and whether the article opened a sentence.
struct Reference {
    start: usize,
    name: (usize, usize),
    capitalized: bool,
}

fn reference_before(line: &str, at: usize) -> Option<Reference> {
    // `tools` and `toolkit` are prose about tools, not a reference to one.
    if line[at + 4..]
        .chars()
        .next()
        .is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let head = line[..at].strip_suffix(' ')?.trim_end();
    let (name, outer) = match head.strip_suffix('`') {
        Some(quoted) => {
            let open = quoted.rfind('`')?;
            ((open + 1, head.len() - 1), open)
        }
        None => {
            let start = head
                .char_indices()
                .rev()
                .take_while(|(_, ch)| word_char(*ch))
                .last()
                .map_or(head.len(), |(start, _)| start);
            ((start, head.len()), start)
        }
    };
    if name.0 >= name.1 {
        return None;
    }
    let before = line[..outer].trim_end();
    let article = ["the", "The"]
        .iter()
        .find(|article| {
            before.ends_with(**article) && !line[..before.len() - 3].ends_with(word_char)
        })
        .map(|article| (before.len() - article.len(), article.starts_with('T')));
    Some(Reference {
        start: article.map_or(outer, |(start, _)| start),
        name,
        capitalized: article.is_some_and(|(_, capital)| capital),
    })
}

fn word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Worth naming in a warning: an MCP id or a PascalCase identifier — the
/// shapes a tool name takes. "the right tool for the job" is prose.
fn tool_shaped(name: &str) -> bool {
    name.starts_with("mcp__") || name.chars().any(|ch| ch.is_ascii_uppercase())
}

fn remember(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|kept| kept == name) {
        list.push(name.to_owned());
    }
}

/// Codex's phrase swallows the article, so a reference that opened a
/// sentence must not leave it lowercase.
fn capitalize(phrase: &str) -> String {
    let mut chars = phrase.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Inline code spans as outer byte ranges. A run of backticks closes only
/// on a run of the same length, so a span may quote backticks of its own.
fn code_spans(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at += 1;
            continue;
        }
        let run = bytes[at..].iter().take_while(|b| **b == b'`').count();
        let mut scan = at + run;
        while scan < bytes.len() {
            if bytes[scan] != b'`' {
                scan += 1;
                continue;
            }
            let close = bytes[scan..].iter().take_while(|b| **b == b'`').count();
            if close == run {
                spans.push((at, scan + close));
                break;
            }
            scan += close;
        }
        at = match spans.last() {
            Some((start, end)) if *start == at => *end,
            _ => at + run,
        };
    }
    spans
}

/// Markdown links as outer byte ranges — link text and target both.
fn link_ranges(line: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for (at, _) in line.match_indices("](") {
        let (Some(open), Some(close)) = (line[..at].rfind('['), line[at + 2..].find(')')) else {
            continue;
        };
        ranges.push((open, at + 3 + close));
    }
    ranges
}

/// A fence line: up to three spaces of indent, then three or more backticks
/// or tildes. `bare` — nothing but whitespace after the run — is what makes
/// a line eligible to close a fence rather than open one.
fn fence_marker(line: &str) -> Option<(char, usize, bool)> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let marker = rest.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let run = rest.chars().take_while(|c| *c == marker).count();
    (run >= 3).then(|| (marker, run, rest[run..].trim().is_empty()))
}

#[cfg(test)]
mod tests;
