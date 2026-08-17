use std::path::Path;

use crate::model::HarnessId;

/// A hook source: shell script with YAML-in-comments frontmatter between
/// `# ---` delimiter lines (v1 format, preserved verbatim).
#[derive(Debug, Clone, PartialEq)]
pub struct HookSource {
    pub name: String,
    pub event: String,
    pub matcher: Option<String>,
    pub description: String,
    pub safety: Option<String>,
    pub timeout: Option<u32>,
    /// Harness allowlist; `None` = every harness.
    pub harnesses: Option<Vec<String>>,
    pub script: String,
}

impl HookSource {
    pub fn applies_to(&self, harness: HarnessId) -> bool {
        match &self.harnesses {
            None => true,
            Some(list) => list.iter().any(|h| HarnessId::parse(h) == Some(harness)),
        }
    }

    /// Advisory prose for harnesses without native hook support (codex
    /// unmapped events, cursor rules).
    pub fn safety_prose(&self) -> String {
        let mut prose = format!("**Safety: {}**\n", self.description);
        if let Some(safety) = &self.safety {
            prose.push_str(safety);
            prose.push('\n');
        }
        let action = match self.event.as_str() {
            "PreToolUse" => "Before executing",
            "PostToolUse" => "After executing",
            "PermissionRequest" => "When requesting permission for",
            "PostCompact" => "After context compaction",
            "TaskCompleted" => "Before marking a task complete",
            _ => "When handling",
        };
        let target = self.matcher.as_deref().unwrap_or("any tool");
        prose.push_str(&format!(
            "{action} {target} operations, the agent must verify this constraint is met.\n"
        ));
        prose
    }
}

/// One hook as a harness registers it: the harness's own event name, and the
/// matcher in the harness's own tool names.
#[derive(Debug, Clone, PartialEq)]
pub struct Registration {
    pub hook: HookSource,
    /// The matcher carries regex syntax around a tool name, so it is
    /// registered exactly as authored and may match nothing here.
    pub matcher_as_authored: bool,
}

impl Registration {
    pub fn new(source: &HookSource, harness: HarnessId, event: &str) -> Registration {
        let said = source
            .matcher
            .as_deref()
            .map(|matcher| crate::render::vocab::hook_matcher(matcher, harness));
        Registration {
            hook: HookSource {
                event: event.to_owned(),
                matcher: said.as_ref().map(|(pattern, _)| pattern.clone()),
                ..source.clone()
            },
            matcher_as_authored: said.is_some_and(|(_, said)| !said),
        }
    }
}

pub fn parse_hook(text: &str) -> Result<HookSource, String> {
    let mut in_frontmatter = false;
    let mut seen_frontmatter = false;
    let mut hook = HookSource {
        name: String::new(),
        event: String::new(),
        matcher: None,
        description: String::new(),
        safety: None,
        timeout: None,
        harnesses: None,
        script: text.to_owned(),
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "# ---" {
            if in_frontmatter {
                seen_frontmatter = true;
                break;
            }
            in_frontmatter = true;
            continue;
        }
        if !in_frontmatter {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix('#') else {
            continue;
        };
        let Some((key, value)) = rest.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "name" => hook.name = value.to_owned(),
            "event" => hook.event = value.to_owned(),
            "matcher" => hook.matcher = Some(value.to_owned()),
            "description" => hook.description = value.to_owned(),
            "safety" => hook.safety = Some(value.to_owned()),
            "timeout" => hook.timeout = value.parse().ok(),
            "harnesses" => {
                let list: Vec<String> = value
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(|h| h.trim().trim_matches('"').to_owned())
                    .filter(|h| !h.is_empty())
                    .collect();
                if !list.is_empty() {
                    hook.harnesses = Some(list);
                }
            }
            _ => {}
        }
    }
    if !seen_frontmatter {
        return Err("hook script has no `# ---` frontmatter block".to_owned());
    }
    if hook.name.is_empty() || hook.event.is_empty() {
        return Err("hook frontmatter needs at least name and event".to_owned());
    }
    Ok(hook)
}

/// v1's codex event mapping: identity for the events codex understands,
/// `None` for events that fall back to advisory prose in agent files.
pub fn codex_event(event: &str) -> Option<&str> {
    match event {
        "SessionStart" | "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "PreCompact"
        | "PostCompact" | "PermissionRequest" | "Stop" => Some(event),
        _ => None,
    }
}

/// A short recognizable handle for a shell command: the file stem of its
/// script-looking token, else the first token.
pub fn command_stem(command: &str) -> String {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let script = tokens
        .iter()
        .map(|t| t.trim_matches('"').trim_matches('\''))
        .find(|t| t.contains('/') || t.contains('.'));
    let pick = script.or(tokens.first().copied()).unwrap_or(command);
    Path::new(pick)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(pick)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = "#!/usr/bin/env bash\n# ---\n# name: guard\n# event: PreToolUse\n# matcher: Bash\n# description: block dangerous commands\n# timeout: 10\n# harnesses: [claude-code, codex]\n# ---\nexit 0\n";

    #[test]
    fn parses_v1_comment_frontmatter() {
        let hook = parse_hook(SCRIPT).unwrap();
        assert_eq!(hook.name, "guard");
        assert_eq!(hook.event, "PreToolUse");
        assert_eq!(hook.matcher.as_deref(), Some("Bash"));
        assert_eq!(hook.timeout, Some(10));
        assert!(hook.applies_to(HarnessId::Claude));
        assert!(hook.applies_to(HarnessId::Codex));
        assert!(!hook.applies_to(HarnessId::Pi));
        assert!(hook.script.contains("exit 0"));
    }

    #[test]
    fn missing_frontmatter_or_fields_is_an_error() {
        assert!(parse_hook("#!/bin/sh\nexit 0\n").is_err());
        assert!(parse_hook("# ---\n# name: x\n# ---\n").is_err());
    }

    #[test]
    fn codex_event_mapping_matches_v1() {
        assert_eq!(codex_event("PreToolUse"), Some("PreToolUse"));
        assert_eq!(codex_event("TaskCompleted"), None);
    }
}
