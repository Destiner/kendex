use super::*;

use crate::source::plugin_registry::read;

const REGISTRY: &str = r#"{
  "name": "workflows",
  "owner": {"name": "wshobson"},
  "metadata": {"description": "workflows", "version": "1.2.0"},
  "plugins": [
    {"name": "data-science", "source": "./plugins/data-science", "version": "0.4.0",
     "description": "analysis", "category": "analysis", "license": "MIT",
     "author": {"name": "wshobson"}, "homepage": "https://example.invalid"},
    {"name": "code-review", "source": "./plugins/code-review", "version": "1.0.0"}
  ]
}"#;

fn write(root: &std::path::Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// A wshobson-shaped catalog: a registry, two plugins, three kinds.
fn fixture() -> (tempfile::TempDir, SealedSource) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("catalog");
    write(&root, ".claude-plugin/marketplace.json", REGISTRY);
    write(
        &root,
        "plugins/data-science/.claude-plugin/plugin.json",
        r#"{"name": "data-science", "version": "0.4.0", "skills": "./skills"}"#,
    );
    write(
        &root,
        "plugins/data-science/agents/eda.md",
        "---\nname: eda\ndescription: explore\n---\nbody\n",
    );
    write(
        &root,
        "plugins/data-science/skills/eda/SKILL.md",
        "---\nname: eda\ndescription: explore\n---\nbody\n",
    );
    write(
        &root,
        "plugins/data-science/commands/report.md",
        "Write the report.\n",
    );
    write(
        &root,
        "plugins/code-review/agents/reviewer.md",
        "---\nname: reviewer\ndescription: review\n---\nbody\n",
    );
    let sealed = SealedSource::open(&root).expect("open");
    (tmp, sealed)
}

#[test]
fn items_are_named_for_the_plugin_they_came_from() {
    let (_tmp, sealed) = fixture();
    let registry = read(&sealed)
        .expect("read")
        .expect("plugin-registry-shaped");

    assert_eq!(
        items(&sealed, &registry, ItemKind::Agent),
        ["code-review/reviewer", "data-science/eda"]
    );
    assert_eq!(
        items(&sealed, &registry, ItemKind::Skill),
        ["data-science/eda"]
    );
    assert_eq!(
        items(&sealed, &registry, ItemKind::Command),
        ["data-science/report"]
    );
    // Kinds a plugin has no place for offer nothing rather than guessing.
    assert!(items(&sealed, &registry, ItemKind::Hook).is_empty());
}

#[test]
fn names_resolve_only_through_the_registry() {
    let (_tmp, sealed) = fixture();
    let registry = read(&sealed)
        .expect("read")
        .expect("plugin-registry-shaped");

    let skill = find(&sealed, &registry, ItemKind::Skill, "data-science/eda").expect("found");
    assert!(skill.ends_with("plugins/data-science/skills/eda"));
    let agent = find(&sealed, &registry, ItemKind::Agent, "data-science/eda").expect("found");
    assert!(agent.ends_with("plugins/data-science/agents/eda.md"));

    // A plugin the registry never validated is not a directory to read from,
    // whatever the repository happens to hold.
    assert_eq!(
        find(&sealed, &registry, ItemKind::Agent, "unlisted/x"),
        None
    );
    // Flat names mean nothing here, and neither does climbing out.
    assert_eq!(find(&sealed, &registry, ItemKind::Agent, "eda"), None);
    assert_eq!(
        find(
            &sealed,
            &registry,
            ItemKind::Agent,
            "data-science/../../secret"
        ),
        None
    );
}

#[test]
fn each_plugin_is_a_group_carrying_its_members_and_its_metadata() {
    let (_tmp, sealed) = fixture();
    let meta = metadata(&sealed)
        .expect("read")
        .expect("plugin-registry-shaped");

    assert_eq!(meta.name, "workflows");
    assert_eq!(meta.version.as_deref(), Some("1.2.0"));
    assert_eq!(meta.groups.len(), 2);
    let group = meta
        .groups
        .iter()
        .find(|g| g.name == "data-science")
        .expect("group");
    assert_eq!(group.version.as_deref(), Some("0.4.0"));
    assert_eq!(group.category.as_deref(), Some("analysis"));
    assert_eq!(group.license.as_deref(), Some("MIT"));
    assert_eq!(group.author.as_deref(), Some("wshobson"));
    let members: Vec<&str> = group.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        members,
        [
            "data-science/eda",
            "data-science/report",
            "data-science/eda"
        ]
    );
    assert!(meta.findings.is_empty(), "{:?}", meta.findings);
}

#[test]
fn a_catalog_whose_two_files_disagree_is_reported() {
    let (tmp, _) = fixture();
    let root = tmp.path().join("catalog");
    write(
        &root,
        "plugins/data-science/.claude-plugin/plugin.json",
        r#"{"name": "datascience", "version": "9.9.9", "skills": "../code-review/skills"}"#,
    );
    let sealed = SealedSource::open(&root).expect("open");
    let meta = metadata(&sealed)
        .expect("read")
        .expect("plugin-registry-shaped");
    let problems: Vec<&str> = meta.findings.iter().map(|f| f.problem.as_str()).collect();

    assert!(
        problems.iter().any(|p| p.contains("calls itself")),
        "{problems:?}"
    );
    assert!(problems.iter().any(|p| p.contains("9.9.9")), "{problems:?}");
    assert!(
        problems.iter().any(|p| p.contains("outside this plugin")),
        "{problems:?}"
    );
    for finding in &meta.findings {
        assert!(!finding.fix.is_empty(), "{finding} needs a fix");
    }
}

#[test]
fn item_names_a_filesystem_would_fold_together_are_reported_not_installed() {
    let (tmp, _) = fixture();
    let root = tmp.path().join("catalog");
    write(
        &root,
        "plugins/data-science/agents/EDA.md",
        "---\nname: EDA\n---\nbody\n",
    );
    write(&root, "plugins/data-science/agents/nul.md", "---\n---\n");
    let sealed = SealedSource::open(&root).expect("open");
    let registry = read(&sealed)
        .expect("read")
        .expect("plugin-registry-shaped");
    let meta = metadata(&sealed)
        .expect("read")
        .expect("plugin-registry-shaped");
    let problems: Vec<&str> = meta.findings.iter().map(|f| f.problem.as_str()).collect();

    assert!(
        problems.iter().any(|p| p.contains("folded case")),
        "{problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains("reserved device name")),
        "{problems:?}"
    );
    // The offered list holds one of the two folded names, never both.
    let agents = items(&sealed, &registry, ItemKind::Agent);
    assert!(!agents.iter().any(|name| name.contains("nul")));
}

#[cfg(unix)]
#[test]
fn a_symlinked_plugin_component_is_never_read_through() {
    let (tmp, _) = fixture();
    let root = tmp.path().join("catalog");
    std::fs::write(tmp.path().join("secret.md"), "host secret").expect("write");
    std::os::unix::fs::symlink(
        tmp.path().join("secret.md"),
        root.join("plugins/code-review/agents/leak.md"),
    )
    .expect("symlink");
    let sealed = SealedSource::open(&root).expect("open");
    let registry = read(&sealed)
        .expect("read")
        .expect("plugin-registry-shaped");

    assert_eq!(
        find(&sealed, &registry, ItemKind::Agent, "code-review/leak"),
        None
    );
    assert!(
        !items(&sealed, &registry, ItemKind::Agent)
            .iter()
            .any(|name| name.ends_with("leak"))
    );
}
