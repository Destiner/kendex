use super::Finding;

/// Gemini reads a command as a TOML table: `prompt` is what it runs and is
/// required, `description` is what it lists the command under (matrix §1).
/// A file that does not parse is skipped in silence, and one with no prompt
/// is a command that does nothing when it is typed.
pub(super) fn gemini(text: &str) -> Vec<Finding> {
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(problem) => {
            return vec![Finding::breakage(
                format!("Gemini reads commands as TOML and this one does not parse — {problem}"),
                "check the command's body in the catalog for control characters",
            )];
        }
    };
    let filled = |key: &str| {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    };
    let mut findings = Vec::new();
    if !filled("prompt") {
        findings.push(Finding::breakage(
            "the Gemini command has no prompt, so typing it would do nothing",
            "give the command a body in the catalog",
        ));
    }
    if !filled("description") {
        findings.push(Finding::advisory(
            "the Gemini command has no description, so it lists with nothing beside it",
            "add `description:` to the command's frontmatter, or open its body with a line saying what it does",
        ));
    }
    findings
}
