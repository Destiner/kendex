//! What each harness calls the tools an agent talks about. Agent bodies are
//! authored in Claude's vocabulary — "the Read tool" — and every other
//! harness reads that as a name for something it does not have. This module
//! owns all three translations: the manifest-name tables the renderers use
//! for their permission fields, and the conservative prose rewrite that lets
//! a body say the same thing in the reader's own words.

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

mod rewrite;
pub use rewrite::rewrite_prose;

#[cfg(test)]
mod tests;
