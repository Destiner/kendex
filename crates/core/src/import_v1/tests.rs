use super::*;
const V1_MANIFEST: &str = r#"
[agent-launch-instructions]
generalist = ""
rust = "Read docs/architecture.md before coding."

[agent-guidance]
iced = "Read docs/ui.md."

[agent-skills]
rust = ["dev", "github"]

[skill-instructions]
github = "prefer gh cli"

[agent-colors]
rust = "orange"

[agent-frontmatter.claude-code.rust]
model = "opus"
deny-tools = "WebSearch, WebFetch"
allowedSubagents = ["scout"]
tools = ["Read"]

[agent-frontmatter.legacykey]
color = "red"
tools = ["Read", "Grep"]

[[custom-hooks]]
event = "PreToolUse"
matcher = "Bash"
command = "./guard.sh"
"#;

const V1_LOCK: &str = r#"{
  "version": 1,
  "entries": {
    "decider": {
      "name": "decider", "kind": "skill",
      "source": "vanillagreencom/vstack", "source_repo": "vanillagreencom/vstack",
      "harnesses": ["pi", "claude-code", "codex"],
      "method": "symlink", "installed_at": "2026-08-10T15:42:13Z", "source_hash": "3a368ae2"
    },
    "rust": {
      "name": "rust", "kind": "agent",
      "source": "/home/u/dev/vstack", "source_repo": "vanillagreencom/vstack",
      "harnesses": ["claude-code"],
      "method": "symlink", "installed_at": "2026-08-10T15:42:13Z", "source_hash": "aa"
    },
    "sunset": {
      "name": "sunset", "kind": "extra",
      "source": "vanillagreencom/vstack",
      "harnesses": [], "method": "symlink", "installed_at": "", "source_hash": ""
    }
  }
}"#;

#[test]
fn converts_tables_with_aliases_and_drops_the_dead_ones() {
    let outcome = convert(Some(V1_MANIFEST), None).unwrap();
    let m = &outcome.manifest;
    assert_eq!(m.schema, crate::manifest::MANIFEST_SCHEMA);
    assert_eq!(
        m.agent_launch_instructions.get("rust").map(String::as_str),
        Some("Read docs/architecture.md before coding.")
    );
    // The `agent-guidance` alias merges; empty strings drop.
    assert!(m.agent_launch_instructions.contains_key("iced"));
    assert!(!m.agent_launch_instructions.contains_key("generalist"));
    assert_eq!(m.agent_skills["rust"], ["dev", "github"]);
    let overrides = &m.agent_frontmatter["claude"]["rust"];
    assert_eq!(overrides.model.as_deref(), Some("opus"));
    assert_eq!(
        overrides.deny_tools,
        Some(vec!["WebSearch".to_owned(), "WebFetch".to_owned()])
    );
    assert_eq!(overrides.allowed_subagents, Some(vec!["scout".to_owned()]));
    // The v1 `tools` allowlist survives as allow-only intent — dropping it
    // would migrate a restricted agent unrestricted.
    assert_eq!(overrides.allow_tools, Some(vec!["Read".to_owned()]));
    assert_eq!(m.custom_hooks.len(), 1);
    let joined = outcome.notes.join("\n");
    assert!(joined.contains("agent-colors"));
    assert!(joined.contains("legacykey"));
    assert!(joined.contains("tools"));
}

#[test]
fn harness_agnostic_overrides_expand_to_every_harness() {
    let outcome = convert(Some(V1_MANIFEST), None).unwrap();
    let m = &outcome.manifest;
    for harness in crate::model::HarnessId::ALL {
        let overrides = &m.agent_frontmatter[harness.name()]["legacykey"];
        assert_eq!(
            overrides.color.as_deref(),
            Some("red"),
            "{}",
            harness.name()
        );
        assert_eq!(
            overrides.allow_tools,
            Some(vec!["Read".to_owned(), "Grep".to_owned()]),
            "{}",
            harness.name()
        );
    }
    assert!(
        outcome
            .notes
            .iter()
            .any(|n| n.contains("expanded harness-agnostic"))
    );
}

#[test]
fn lock_entries_split_per_harness_and_extras_are_skipped() {
    let outcome = convert(None, Some(V1_LOCK)).unwrap();
    assert_eq!(outcome.lock.entries.len(), 4);
    assert!(outcome.lock.entries.contains_key("skill:decider:pi"));
    assert!(outcome.lock.entries.contains_key("skill:decider:claude"));
    assert!(outcome.lock.entries.contains_key("agent:rust:claude"));
    // Declarations + the default source derive from the lock.
    assert_eq!(outcome.manifest.skills["decider"].source, "vstack");
    assert_eq!(outcome.manifest.agents["rust"].source, "vstack");
    assert_eq!(
        outcome.manifest.sources["vstack"].repo.as_deref(),
        Some("vanillagreencom/vstack")
    );
    assert!(outcome.notes.iter().any(|n| n.contains("sunset")));
    // Imported hashes never match recomputed ones → first refresh
    // regenerates.
    assert!(
        outcome.lock.entries["skill:decider:pi"]
            .source_hash
            .starts_with("v1:")
    );
}

#[test]
fn settings_seeds_import_with_owner_when_one_skill_is_installed() {
    let lock = r#"{
        "version": 1,
        "entries": {
            "decider": { "kind": "skill", "source": "vanillagreencom/vstack",
                         "harnesses": ["claude-code"], "installed_at": "t", "source_hash": "x" }
        },
        "settings_seeds": { "REVIEWERS": "cbf29ce484222325" }
    }"#;
    let outcome = convert(None, Some(lock)).unwrap();
    let record = outcome.lock.settings_seeds.get("REVIEWERS").unwrap();
    // One installed skill: every seeded key is unambiguously its.
    assert_eq!(record.owner.as_deref(), Some("decider"));
    // Hash-for-hash: same algorithm, so migrated repos keep refreshing
    // instead of re-freezing.
    assert_eq!(record.hash, "cbf29ce484222325");
}

#[test]
fn contested_settings_seeds_import_legacy_owned() {
    let lock = r#"{
        "version": 1,
        "entries": {
            "one": { "kind": "skill", "source": "vanillagreencom/vstack",
                     "harnesses": ["claude-code"], "installed_at": "t", "source_hash": "x" },
            "two": { "kind": "skill", "source": "vanillagreencom/vstack",
                     "harnesses": ["claude-code"], "installed_at": "t", "source_hash": "x" }
        },
        "settings_seeds": { "REVIEWERS": "cbf29ce484222325" }
    }"#;
    let outcome = convert(None, Some(lock)).unwrap();
    let record = outcome.lock.settings_seeds.get("REVIEWERS").unwrap();
    assert_eq!(record.owner, None, "contested keys import legacy-owned");
    assert!(
        outcome
            .notes
            .iter()
            .any(|n| n.contains("never auto-refreshed")),
        "{:?}",
        outcome.notes
    );
}
