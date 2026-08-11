use serde::Serialize;
use specta::Type;
use vstack_core::apply::{Op, PlannedOp, Pre};
use vstack_core::engine::{self, PlanOptions, ops};
use vstack_core::env::Env;
use vstack_core::lock::{load as load_lock, lock_path};
use vstack_core::manifest::{self, Finding, Manifest};
use vstack_core::model::{HarnessId, ItemKind, Scope};
use vstack_core::source::{self, SourceState};
use vstack_core::{apply, manifest::LOCAL_SOURCE_NAME};

use crate::audit::{AuditView, view};

fn env() -> Result<Env, String> {
    Env::detect().map_err(|e| e.to_string())
}

/// What the Customize page needs to offer real choices: the names already
/// declared here plus the skills any ready source can supply.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EditorInventory {
    pub declared_agents: Vec<String>,
    pub declared_skills: Vec<String>,
    pub available_skills: Vec<String>,
    pub harnesses: Vec<HarnessId>,
}

#[tauri::command]
#[specta::specta]
pub fn get_manifest(scope: Scope) -> Result<Option<Manifest>, String> {
    let env = env()?;
    manifest::load_for_mutation(&manifest::manifest_path(&env, &scope)).map_err(|e| e.to_string())
}

/// Validate an edited manifest the way a hand-written file is validated, so
/// the editor rejects exactly the same things — fix strings included.
fn check(manifest: &Manifest) -> Result<(), String> {
    let text = toml::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let findings = manifest::validate(&table);
    if findings.is_empty() {
        return Ok(());
    }
    Err(findings
        .iter()
        .map(Finding::to_string)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// The editor can create the first manifest for a scope, and first creation
/// is where the default source is seeded — skipping it here would drop it
/// for good, since later reconciliation never re-adds it.
fn on_first_creation(mut manifest: Manifest, seed: Manifest) -> Manifest {
    if manifest.sources.is_empty() {
        manifest.sources = seed.sources;
        if manifest.install.harnesses.is_empty() {
            manifest.install.harnesses = seed.install.harnesses;
        }
    }
    manifest
}

/// Write an edited manifest and reconcile the scope to it.
#[tauri::command]
#[specta::specta]
pub fn update_manifest(scope: Scope, manifest: Manifest) -> Result<AuditView, String> {
    let env = env()?;
    let path = manifest::manifest_path(&env, &scope);
    let manifest = match manifest::load_for_mutation(&path).map_err(|e| e.to_string())? {
        Some(_) => manifest,
        None => on_first_creation(
            manifest,
            ops::manifest_for_mutation(&env, &scope).map_err(|e| e.to_string())?,
        ),
    };
    check(&manifest)?;
    let lock = load_lock(&lock_path(&env, &scope)).map_err(|e| e.to_string())?;
    let mut report = engine::plan_scope(&env, &scope, &manifest, &lock, &PlanOptions::default())
        .map_err(|e| e.to_string())?;
    let persisted = report
        .plan
        .ops
        .iter()
        .any(|op| matches!(op.op, Op::WriteManifest { .. }));
    if !persisted {
        report.plan.ops.insert(
            0,
            PlannedOp {
                description: "write vstack.toml".into(),
                op: Op::WriteManifest {
                    pre: Pre::observed(&path).map_err(|e| e.to_string())?,
                    path: path.clone(),
                    manifest: Box::new(manifest),
                },
            },
        );
    }
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    view(&env, &scope)
}

#[tauri::command]
#[specta::specta]
pub fn editor_inventory(scope: Scope) -> Result<EditorInventory, String> {
    let env = env()?;
    let loaded = manifest::load_for_mutation(&manifest::manifest_path(&env, &scope))
        .map_err(|e| e.to_string())?;
    let mut inventory = EditorInventory {
        declared_agents: Vec::new(),
        declared_skills: Vec::new(),
        available_skills: Vec::new(),
        harnesses: HarnessId::ALL.to_vec(),
    };
    let Some(manifest) = loaded else {
        return Ok(inventory);
    };
    inventory.declared_agents = manifest.agents.keys().cloned().collect();
    inventory.declared_skills = manifest.skills.keys().cloned().collect();

    let mut available: Vec<String> = Vec::new();
    let names = manifest
        .sources
        .keys()
        .map(String::as_str)
        .chain(std::iter::once(LOCAL_SOURCE_NAME));
    for name in names {
        let resolved = source::resolve(&env, &scope, name, &manifest).map_err(|e| e.to_string())?;
        let SourceState::Ready(ready) = resolved else {
            continue;
        };
        let config = source::source_config(&ready.root).map_err(|e| e.to_string())?;
        available.extend(source::list_items(&ready.root, &config, ItemKind::Skill));
    }
    available.sort();
    available.dedup();
    inventory.available_skills = available;
    Ok(inventory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use vstack_core::manifest::{FrontmatterOverrides, ItemDecl, MANIFEST_SCHEMA, SourceDecl};

    fn manifest() -> Manifest {
        Manifest {
            schema: MANIFEST_SCHEMA,
            sources: BTreeMap::from([(
                "vstack".to_owned(),
                SourceDecl {
                    repo: Some("vanillagreencom/vstack".to_owned()),
                    path: None,
                    enabled: true,
                },
            )]),
            ..Manifest::default()
        }
    }

    #[test]
    fn customization_tables_pass_the_same_check_a_file_gets() {
        let mut edited = manifest();
        edited
            .agent_skills
            .insert("orch".to_owned(), vec!["github".to_owned()]);
        edited
            .agent_launch_instructions
            .insert("all".to_owned(), "read the plan".to_owned());
        edited.custom_hooks.push(vstack_core::manifest::CustomHook {
            event: "PreToolUse".to_owned(),
            matcher: Some("Bash".to_owned()),
            command: "./guard.sh".to_owned(),
            description: None,
            agents: vstack_core::manifest::HookAgents::One("all".to_owned()),
        });
        assert_eq!(check(&edited), Ok(()));
    }

    /// `project-skills-dir` is a bare key declared after the tables, so it
    /// only survives if serialization emits values before them.
    #[test]
    fn a_project_skills_dir_survives_the_round_trip() {
        let mut edited = manifest();
        edited.project_skills_dir = Some(".claude/skills-src".to_owned());
        edited.agent_frontmatter.insert(
            "claude".to_owned(),
            BTreeMap::from([("orch".to_owned(), FrontmatterOverrides::default())]),
        );
        assert_eq!(check(&edited), Ok(()));

        let text = toml::to_string_pretty(&edited).unwrap();
        let reparsed: Manifest = toml::from_str(&text).unwrap();
        assert_eq!(reparsed.project_skills_dir, edited.project_skills_dir);
    }

    #[test]
    fn creating_a_manifest_here_still_seeds_the_default_source() {
        let seeded = on_first_creation(
            Manifest {
                schema: MANIFEST_SCHEMA,
                ..Manifest::default()
            },
            manifest::seed(&[HarnessId::Claude]),
        );
        assert!(seeded.sources.contains_key("vstack"));
        assert_eq!(seeded.install.harnesses, [HarnessId::Claude]);

        let declared = on_first_creation(manifest(), manifest::seed(&[HarnessId::Pi]));
        assert_eq!(declared.sources.len(), 1);
        assert!(declared.install.harnesses.is_empty());
    }

    #[test]
    fn rejected_edits_come_back_with_their_fix_string() {
        let mut edited = manifest();
        edited
            .skills
            .insert("github".to_owned(), ItemDecl::from_source("gone"));
        let error = check(&edited).expect_err("undeclared source must be rejected");
        assert!(error.contains("skills.github"), "{error}");
        assert!(error.contains("fix: declare [sources.gone]"), "{error}");
    }
}
