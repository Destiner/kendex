//! `vstack.settings.toml` seeding — skills ship a
//! `vstack.settings.toml.example` and their `[env]` entries merge into the
//! project's settings file, write-if-absent per key (v1 semantics kept
//! line-for-line: comment blocks travel with their key, and uniqueness is
//! file-wide because the shell-side reader is not TOML-table-aware).

pub const SETTINGS_FILE: &str = "vstack.settings.toml";
pub const SETTINGS_TEMPLATE: &str = "vstack.settings.toml.example";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    pub key: String,
    pub lines: Vec<String>,
}

fn is_table_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('[') && trimmed.ends_with(']')
}

fn is_env_header(line: &str) -> bool {
    line.trim() == "[env]"
}

fn assignment_key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.is_empty() || is_table_header(trimmed) {
        return None;
    }
    trimmed
        .split_once('=')
        .map(|(key, _)| key.trim().trim_matches('"').to_owned())
        .filter(|k| !k.is_empty())
}

pub fn extract_env_entries(template: &str) -> Vec<EnvEntry> {
    let mut entries = Vec::new();
    let mut in_env = false;
    let mut pending: Vec<String> = Vec::new();
    for line in template.lines() {
        if is_table_header(line) {
            if is_env_header(line) {
                in_env = true;
                pending.clear();
                continue;
            }
            if in_env {
                break;
            }
        }
        if !in_env {
            continue;
        }
        if line.trim().is_empty() {
            pending.clear();
            continue;
        }
        if line.trim().starts_with('#') {
            pending.push(line.to_owned());
            continue;
        }
        if let Some(key) = assignment_key(line) {
            let mut lines = std::mem::take(&mut pending);
            lines.push(line.to_owned());
            entries.push(EnvEntry { key, lines });
        }
    }
    entries
}

pub fn assigned_keys(text: &str) -> Vec<String> {
    text.lines().filter_map(assignment_key).collect()
}

fn render_entries(entries: &[EnvEntry]) -> String {
    let mut out = String::new();
    for entry in entries {
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
        for line in &entry.lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Merge missing entries into the settings text. `None` = nothing to add.
/// Returns the new text plus the keys that were added.
pub fn merge(original: Option<&str>, entries: &[EnvEntry]) -> Option<(String, Vec<String>)> {
    let existing = original.map(assigned_keys).unwrap_or_default();
    let missing: Vec<&EnvEntry> = entries
        .iter()
        .filter(|entry| !existing.contains(&entry.key))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let added: Vec<String> = missing.iter().map(|e| e.key.clone()).collect();
    let owned: Vec<EnvEntry> = missing.into_iter().cloned().collect();

    let Some(original) = original else {
        let mut out = String::from(
            "# Public vstack settings seeded from installed skill defaults.\n# Skill scripts read this [env] table after .env and before .env.local.\n# Keep secrets, tokens, and personal overrides in .env.local.\n\n[env]\n",
        );
        out.push_str(&render_entries(&owned));
        return Some((out, added));
    };

    let lines: Vec<String> = original.lines().map(str::to_owned).collect();
    let Some(env_start) = lines.iter().position(|l| is_env_header(l)) else {
        let mut out = original.to_owned();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str("[env]\n");
        out.push_str(&render_entries(&owned));
        return Some((out, added));
    };
    let env_end = lines
        .iter()
        .enumerate()
        .skip(env_start + 1)
        .find_map(|(index, line)| is_table_header(line).then_some(index))
        .unwrap_or(lines.len());

    let mut block: Vec<String> = render_entries(&owned)
        .trim_end_matches('\n')
        .lines()
        .map(str::to_owned)
        .collect();
    if env_end > 0 && !lines[env_end - 1].trim().is_empty() {
        block.insert(0, String::new());
    }
    if env_end < lines.len() && !block.last().is_some_and(|l| l.trim().is_empty()) {
        block.push(String::new());
    }
    let mut merged = lines;
    merged.splice(env_end..env_end, block);
    let mut out = merged.join("\n");
    out.push('\n');
    Some((out, added))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "# ignored preamble\n[env]\n# Which reviewer set to run.\n# Comma separated.\nREVIEWERS = \"arch,security\"\n\nDEPTH = \"2\"\n\n[other]\nX = \"1\"\n";

    #[test]
    fn entries_carry_their_comment_blocks() {
        let entries = extract_env_entries(TEMPLATE);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "REVIEWERS");
        assert_eq!(entries[0].lines.len(), 3);
        assert_eq!(entries[1].key, "DEPTH");
    }

    #[test]
    fn merge_is_write_if_absent_with_file_wide_uniqueness() {
        let entries = extract_env_entries(TEMPLATE);
        // Key already assigned under a DIFFERENT table still blocks the add.
        let existing = "[custom]\nREVIEWERS = \"mine\"\n\n[env]\nOTHER = \"x\"\n";
        let (merged, added) = merge(Some(existing), &entries).unwrap();
        assert_eq!(added, ["DEPTH"]);
        assert!(merged.contains("REVIEWERS = \"mine\""));
        assert!(!merged.contains("arch,security"));
        assert!(merged.contains("DEPTH = \"2\""));
        // The new key lands inside [env], before [custom]… order: env is
        // second here, so it lands at the file end.
        assert!(merged.ends_with("DEPTH = \"2\"\n"));

        assert!(merge(Some(&merged), &entries).is_none());
    }

    #[test]
    fn fresh_file_gets_the_seeded_header() {
        let entries = extract_env_entries(TEMPLATE);
        let (created, added) = merge(None, &entries).unwrap();
        assert_eq!(added, ["REVIEWERS", "DEPTH"]);
        assert!(created.starts_with("# Public vstack settings"));
        assert!(created.contains("[env]\n# Which reviewer set to run."));
    }
}
