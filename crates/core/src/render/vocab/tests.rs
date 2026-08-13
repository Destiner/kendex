use super::*;

fn rewrite(body: &str, harness: HarnessId) -> String {
    rewrite_prose(body, harness).0
}

#[test]
fn manifest_names_keep_their_claude_and_opencode_spelling() {
    assert_eq!(claude_tool_name("web_search"), "WebSearch");
    assert_eq!(claude_tool_name("mcp__gh"), "mcp__gh");
    assert_eq!(opencode_permission("apply-patch").as_deref(), Some("edit"));
    assert_eq!(opencode_permission("mcp__gh").as_deref(), Some("mcp__gh"));
    assert_eq!(opencode_permission("  "), None);
}

#[test]
fn a_tool_reference_speaks_each_harness_vocabulary() {
    let body = "Use the Read tool first, then the Bash tool.\n";
    assert_eq!(
        rewrite(body, HarnessId::Opencode),
        "Use the read tool first, then the bash tool.\n"
    );
    assert_eq!(
        rewrite(body, HarnessId::Pi),
        "Use the read tool first, then the bash tool.\n"
    );
    assert_eq!(
        rewrite(body, HarnessId::Cursor),
        "Use the read tool first, then the bash tool.\n"
    );
    // Codex names actions, so the whole reference goes.
    assert_eq!(
        rewrite(body, HarnessId::Codex),
        "Use open the file first, then run a shell command.\n"
    );
    // Bodies are authored in Claude's words already.
    assert_eq!(rewrite(body, HarnessId::Claude), body);
    assert!(rewrite_prose(body, HarnessId::Claude).1.is_empty());
}

#[test]
fn a_reference_that_opens_a_sentence_keeps_its_capital() {
    assert_eq!(
        rewrite("The Grep tool is fast.\n", HarnessId::Codex),
        "Search is fast.\n"
    );
    assert_eq!(
        rewrite("The `Write` tool overwrites.\n", HarnessId::Codex),
        "Edit the file overwrites.\n"
    );
    assert_eq!(
        rewrite("The `Write` tool overwrites.\n", HarnessId::Opencode),
        "The `write` tool overwrites.\n"
    );
}

#[test]
fn one_warning_names_every_tool_reworded_for_the_harness() {
    let body = "the Read tool, the Read tool again, the Grep tool\n";
    let (_, warnings) = rewrite_prose(body, HarnessId::Opencode);
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].message,
        "tool references reworded for OpenCode: Read, Grep"
    );
    assert_eq!(warnings[0].remediation, None);
}

#[test]
fn code_links_and_skill_paths_keep_every_byte() {
    let body = concat!(
        "```\nuse the Read tool\n```\n",
        "~~~md\nuse the Read tool\n~~~\n",
        "````\n```\nuse the Read tool\n```\n````\n",
        "Run `use the Read tool` verbatim.\n",
        "See [the Read tool](https://example.com/the-Read-tool).\n",
        "- dev: .agents/skills/dev/SKILL.md — read it with the Read tool\n",
    );
    for harness in [HarnessId::Codex, HarnessId::Opencode, HarnessId::Cursor] {
        let (text, warnings) = rewrite_prose(body, harness);
        assert_eq!(text, body, "{harness:?} rewrote protected text");
        assert!(
            warnings.is_empty(),
            "{harness:?} warned about protected text"
        );
    }
}

#[test]
fn an_unclosed_fence_protects_the_rest_of_the_body() {
    let body = "```\nuse the Read tool\nstill fenced: the Bash tool\n";
    assert_eq!(rewrite(body, HarnessId::Opencode), body);
}

#[test]
fn unknown_and_mcp_references_pass_through_with_one_warning_each() {
    let body =
        "Call the mcp__github__search tool, the SendMessage tool, the mcp__github__search tool.\n";
    let (text, warnings) = rewrite_prose(body, HarnessId::Codex);
    assert_eq!(text, body);
    assert_eq!(warnings.len(), 2);
    assert_eq!(
        warnings[0].message,
        "`mcp__github__search` is not a Codex tool name — the reference passes through as written"
    );
    assert!(warnings[1].message.starts_with("`SendMessage`"));
}

#[test]
fn a_tool_codex_has_no_word_for_is_reported_not_guessed_at() {
    let body = "Track it with the TodoWrite tool.\n";
    let (text, warnings) = rewrite_prose(body, HarnessId::Codex);
    assert_eq!(text, body);
    assert!(warnings[0].message.contains("`TodoWrite`"));
    // OpenCode has no word for it either, and says so rather than inventing one.
    let (text, warnings) = rewrite_prose(body, HarnessId::Opencode);
    assert_eq!(text, body);
    assert!(warnings[0].message.contains("`TodoWrite`"));
}

#[test]
fn prose_about_tools_is_never_mistaken_for_a_reference() {
    for body in [
        "Pick the right tool for the job.\n",
        "Prefer the dedicated tools over shell commands.\n",
        "The toolkit is yours.\n",
        "the Read toolbox\n",
    ] {
        let (text, warnings) = rewrite_prose(body, HarnessId::Codex);
        assert_eq!(text, body);
        assert!(warnings.is_empty(), "{body} warned");
    }
}

#[test]
fn rewriting_rewritten_text_changes_nothing() {
    let body = concat!(
        "Use the Read tool, the `Grep` tool, and the Bash tool.\n",
        "The Write tool overwrites; the mcp__gh tool does not.\n",
        "```\nthe Read tool\n```\n",
    );
    for harness in [
        HarnessId::Codex,
        HarnessId::Opencode,
        HarnessId::Cursor,
        HarnessId::Pi,
    ] {
        let once = rewrite(body, harness);
        assert_eq!(rewrite(&once, harness), once, "{harness:?} is not stable");
    }
}
