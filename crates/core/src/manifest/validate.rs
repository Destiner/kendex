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

/// What one `[sources.<name>]` table may hold. `rev` names the revision of
/// a remote to read — a commit id pins, a tag or branch tracks.
const SOURCE_KEYS: &[&str] = &["repo", "path", "rev", "enabled"];

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

/// The kinds a marketplace-shaped catalog offers, and so the only ones whose
/// names may carry the plugin they came from. A hook or a server has no
/// namespaced spelling anywhere — a `/` in one of those names would just be
/// a directory on disk that nothing knows to remove.
const NAMESPACED_TABLES: &[&str] = &["agents", "commands", "skills"];

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

/// The tools whose plugin switch vstack can write. Naming any other one
/// asks for a write that has nowhere to land.
fn plugin_harnesses() -> Vec<&'static str> {
    crate::model::HarnessId::ALL
        .into_iter()
        .filter(|h| {
            let toggle = crate::harness::capabilities(*h, crate::model::ItemKind::Plugin).toggle;
            toggle.project || toggle.global
        })
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
        for key in decl.keys() {
            if !SOURCE_KEYS.contains(&key.as_str()) {
                findings.push(Finding {
                    location: location.clone(),
                    problem: format!("unknown key '{key}'"),
                    fix: format!("remove it, or use one of: {}", SOURCE_KEYS.join(", ")),
                });
            }
        }
        let has_repo = decl.get("repo").is_some_and(|v| v.is_str());
        let has_path = decl.get("path").is_some_and(|v| v.is_str());
        if has_repo == has_path {
            findings.push(Finding {
                location: location.clone(),
                problem: "a source needs exactly one of repo or path".into(),
                fix: "keep either repo = \"owner/repo\" or path = \"…\", not both or neither"
                    .into(),
            });
        }
        if let Some(rev) = decl.get("rev") {
            if !rev.is_str() {
                findings.push(Finding {
                    location: location.clone(),
                    problem: "rev must be a string".into(),
                    fix: "write rev = \"<commit, tag or branch>\"".into(),
                });
            } else if !has_repo {
                findings.push(Finding {
                    location,
                    problem: "only a repo has revisions".into(),
                    fix: "remove rev, or point this source at repo = \"owner/repo\"".into(),
                });
            }
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
            // legitimate shape. Every other name becomes a file or a
            // directory, and the only `/` one may hold is the plugin a
            // marketplace-shaped catalog keeps the item in.
            let scoped_ok = kind_table == "pi-extensions"
                && name.starts_with('@')
                && name.matches('/').count() == 1
                && !name.ends_with('/');
            let namespaced = NAMESPACED_TABLES.contains(&kind_table);
            let problem = match (scoped_ok, namespaced) {
                (true, _) => None,
                (false, true) => crate::names::item_problem(name),
                (false, false) => crate::names::segment_problem(name),
            };
            if let Some(problem) = problem {
                findings.push(Finding {
                    location: location.clone(),
                    problem,
                    fix: match namespaced {
                        true => "rename the item — a plain name, or `<plugin>/<item>` for an item from a marketplace catalog".into(),
                        false => format!(
                            "rename the item — a {} is named without a `/`, since no marketplace catalog offers one",
                            kind_table.strip_suffix('s').unwrap_or(kind_table)
                        ),
                    },
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
        let decl = decl.as_table();
        let well_formed = decl.is_some_and(|decl| {
            decl.keys().all(|k| k == "enabled" || k == "harness")
                && decl.get("enabled").is_none_or(Value::is_bool)
        });
        if !well_formed {
            findings.push(Finding {
                location: format!("plugins.{key}"),
                problem: "a plugin declares whether it is enabled and which tool it belongs to"
                    .into(),
                fix: format!("write [plugins.\"{key}\"] with enabled = true or false"),
            });
        }
        // A plugin belongs to one tool, and only some tools have a plugin
        // switch to write at all.
        if let Some(harness) = decl
            .and_then(|decl| decl.get("harness"))
            .and_then(Value::as_str)
            && !plugin_harnesses().contains(&harness)
        {
            findings.push(Finding {
                location: format!("plugins.{key}.harness"),
                problem: format!("{harness} has no plugin switch vstack can write"),
                fix: format!("set harness to one of: {}", plugin_harnesses().join(", ")),
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
mod tests;
