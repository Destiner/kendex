use std::fmt;

use toml::Table;
use toml::Value;

/// One validation problem, always paired with a machine-actionable fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub location: String,
    pub problem: String,
    pub fix: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} — fix: {}", self.location, self.problem, self.fix)
    }
}

const TOP_LEVEL: &[&str] = &[
    "schema",
    "sources",
    "install",
    "agents",
    "skills",
    "hooks",
    "commands",
    "mcp-servers",
    "plugins",
    "pi-extensions",
    "agent-skills",
    "agent-launch-instructions",
    "agent-additional-instructions",
    "skill-instructions",
    "agent-frontmatter",
    "custom-hooks",
    "project-skills-dir",
];

/// Kind tables whose entries name an item from a source. Plugins are not
/// among them: they come from a marketplace and carry only an enabled flag.
const ITEM_TABLES: &[&str] = &[
    "agents",
    "skills",
    "hooks",
    "commands",
    "mcp-servers",
    "pi-extensions",
];

/// The tools a manifest may name: the ones vstack writes to. Read from the
/// capability table rather than listed here, so a tool can never be
/// accepted as a target before anything it declares would be installed.
fn harnesses() -> Vec<&'static str> {
    crate::model::HarnessId::ALL
        .into_iter()
        .filter(|harness| crate::harness::installable(*harness))
        .map(crate::model::HarnessId::name)
        .collect()
}

const FRONTMATTER_KEYS: &[&str] = &[
    "color",
    "model",
    "deny-tools",
    "allow-tools",
    "allowed-subagents",
    "pane",
    "background",
    "effort",
    "isolation",
    "memory",
    "mode",
    "sandbox-mode",
    "model-reasoning-effort",
    "nickname-candidates",
];

pub fn validate(table: &Table) -> Vec<Finding> {
    let mut findings = Vec::new();

    let schema = table.get("schema").and_then(Value::as_integer);
    let readable = i64::from(super::OLDEST_READABLE_SCHEMA)..=i64::from(super::MANIFEST_SCHEMA);
    if !schema.is_some_and(|s| readable.contains(&s)) {
        findings.push(Finding {
            location: "schema".into(),
            problem: "missing or unsupported schema version".into(),
            fix: format!("set schema = {}", super::MANIFEST_SCHEMA),
        });
    }
    for key in table.keys() {
        if !TOP_LEVEL.contains(&key.as_str()) {
            findings.push(Finding {
                location: key.clone(),
                problem: "unknown table or key".into(),
                fix: format!("remove it, or use one of: {}", TOP_LEVEL.join(", ")),
            });
        }
    }
    validate_sources(table, &mut findings);
    validate_install(table, &mut findings);
    validate_items(table, &mut findings);
    validate_plugins(table, &mut findings);
    validate_frontmatter(table, &mut findings);
    validate_hooks(table, &mut findings);
    findings
}

fn validate_sources(table: &Table, findings: &mut Vec<Finding>) {
    let Some(sources) = table.get("sources").and_then(Value::as_table) else {
        return;
    };
    for (name, decl) in sources {
        let location = format!("sources.{name}");
        let Some(decl) = decl.as_table() else {
            findings.push(Finding {
                location,
                problem: "source must be a table".into(),
                fix: "write [sources.<name>] with repo = \"owner/repo\" or path = \"…\"".into(),
            });
            continue;
        };
        let has_repo = decl.get("repo").is_some_and(|v| v.is_str());
        let has_path = decl.get("path").is_some_and(|v| v.is_str());
        if has_repo == has_path {
            findings.push(Finding {
                location,
                problem: "a source needs exactly one of repo or path".into(),
                fix: "keep either repo = \"owner/repo\" or path = \"…\", not both or neither"
                    .into(),
            });
        }
    }
}

fn validate_install(table: &Table, findings: &mut Vec<Finding>) {
    let declared = table
        .get("install")
        .and_then(Value::as_table)
        .and_then(|install| install.get("harnesses"))
        .and_then(Value::as_array);
    for entry in declared.into_iter().flatten() {
        let name = entry.as_str().unwrap_or_default();
        if !harnesses().contains(&name) {
            findings.push(Finding {
                location: "install.harnesses".into(),
                problem: format!("unknown harness '{name}'"),
                fix: format!("use one of: {}", harnesses().join(", ")),
            });
        }
    }
}

fn validate_items(table: &Table, findings: &mut Vec<Finding>) {
    let source_names: Vec<String> = table
        .get("sources")
        .and_then(Value::as_table)
        .map(|s| s.keys().cloned().collect())
        .unwrap_or_default();
    for &kind_table in ITEM_TABLES {
        let Some(items) = table.get(kind_table).and_then(Value::as_table) else {
            continue;
        };
        for (name, decl) in items {
            let location = format!("{kind_table}.{name}");
            // Pi extensions are npm packages, where `@scope/name` is a
            // legitimate shape; everything else keeps flat names.
            let scoped_ok = kind_table == "pi-extensions"
                && name.starts_with('@')
                && name.matches('/').count() == 1
                && !name.ends_with('/');
            if (name.contains('/') && !scoped_ok) || name.starts_with('-') {
                findings.push(Finding {
                    location: location.clone(),
                    problem: "item names must not contain '/' or start with '-'".into(),
                    fix: "rename the item".into(),
                });
            }
            let Some(decl) = decl.as_table() else {
                findings.push(Finding {
                    location,
                    problem: "declaration must be a table".into(),
                    fix: format!("write [{kind_table}.{name}] with source = \"<source-name>\""),
                });
                continue;
            };
            match decl.get("source").and_then(Value::as_str) {
                None => findings.push(Finding {
                    location,
                    problem: "missing source".into(),
                    fix: "add source = \"<source-name>\" (or \"local\")".into(),
                }),
                Some(source) => {
                    if source != super::LOCAL_SOURCE_NAME
                        && !source_names.iter().any(|s| s == source)
                    {
                        findings.push(Finding {
                            location,
                            problem: format!("references undeclared source '{source}'"),
                            fix: format!(
                                "declare [sources.{source}] or change source to one of: {}",
                                if source_names.is_empty() {
                                    "local".to_owned()
                                } else {
                                    format!("{}, local", source_names.join(", "))
                                }
                            ),
                        });
                    }
                }
            }
        }
    }
}

fn validate_plugins(table: &Table, findings: &mut Vec<Finding>) {
    let Some(plugins) = table.get("plugins").and_then(Value::as_table) else {
        return;
    };
    for (key, decl) in plugins {
        let declares_only_enabled = decl.as_table().is_some_and(|decl| {
            decl.keys().all(|k| k == "enabled") && decl.get("enabled").is_none_or(Value::is_bool)
        });
        if !declares_only_enabled {
            findings.push(Finding {
                location: format!("plugins.{key}"),
                problem: "a plugin declares nothing but enabled".into(),
                fix: format!("write [plugins.\"{key}\"] with enabled = true or false"),
            });
        }
    }
}

fn validate_frontmatter(table: &Table, findings: &mut Vec<Finding>) {
    let Some(frontmatter) = table.get("agent-frontmatter").and_then(Value::as_table) else {
        return;
    };
    for (harness, agents) in frontmatter {
        if !harnesses().contains(&harness.as_str()) {
            findings.push(Finding {
                location: format!("agent-frontmatter.{harness}"),
                problem: format!("unknown harness '{harness}'"),
                fix: format!("use one of: {}", harnesses().join(", ")),
            });
            continue;
        }
        let Some(agents) = agents.as_table() else {
            continue;
        };
        for (agent, overrides) in agents {
            let Some(overrides) = overrides.as_table() else {
                continue;
            };
            for key in overrides.keys() {
                if !FRONTMATTER_KEYS.contains(&key.as_str()) {
                    findings.push(Finding {
                        location: format!("agent-frontmatter.{harness}.{agent}.{key}"),
                        problem: "unknown frontmatter override".into(),
                        fix: format!("use one of: {}", FRONTMATTER_KEYS.join(", ")),
                    });
                }
            }
        }
    }
}

fn validate_hooks(table: &Table, findings: &mut Vec<Finding>) {
    let Some(hooks) = table.get("custom-hooks").and_then(Value::as_array) else {
        return;
    };
    for (index, hook) in hooks.iter().enumerate() {
        let Some(hook) = hook.as_table() else {
            continue;
        };
        for required in ["event", "command"] {
            if !hook.get(required).is_some_and(|v| v.is_str()) {
                findings.push(Finding {
                    location: format!("custom-hooks[{index}]"),
                    problem: format!("missing {required}"),
                    fix: format!("add {required} = \"…\""),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Table {
        text.parse().unwrap()
    }

    #[test]
    fn every_finding_carries_a_fix() {
        let table = parse(
            r#"
schema = 99
typo-table = 1

[sources.bad]
enabled = true

[install]
harnesses = ["claude", "emacs"]

[skills.x]
source = "nowhere"

[agents."-bad/name"]
source = "local"

[mcp-servers.gh]
source = "nowhere"

[plugins."fmt@main"]
version = "1"

[agent-frontmatter.claude.orch]
tools = ["a"]

[[custom-hooks]]
matcher = "Bash"
"#,
        );
        let findings = validate(&table);
        let locations: Vec<_> = findings.iter().map(|f| f.location.as_str()).collect();
        assert!(locations.contains(&"mcp-servers.gh"));
        assert!(locations.contains(&"plugins.fmt@main"));
        assert!(locations.contains(&"schema"));
        assert!(locations.contains(&"typo-table"));
        assert!(locations.contains(&"sources.bad"));
        assert!(locations.contains(&"install.harnesses"));
        assert!(locations.contains(&"skills.x"));
        assert!(locations.contains(&"agents.-bad/name"));
        assert!(locations.contains(&"agent-frontmatter.claude.orch.tools"));
        assert!(locations.iter().any(|l| l.starts_with("custom-hooks[0]")));
        for finding in &findings {
            assert!(!finding.fix.is_empty(), "{finding}");
        }
    }

    #[test]
    fn a_clean_manifest_validates_empty() {
        let table = parse(
            r#"
schema = 1
[sources.vstack]
repo = "vanillagreencom/vstack"
[skills.github]
source = "vstack"
[agents.local-one]
source = "local"
[hooks.guard]
source = "vstack"
[mcp-servers.gh]
source = "vstack"
[plugins."fmt@main"]
enabled = false
"#,
        );
        assert_eq!(validate(&table), Vec::new());
    }
}
