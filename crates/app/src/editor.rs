use serde::Serialize;
use specta::Type;
use vstack_core::apply::{Op, PlannedOp, Pre};
use vstack_core::engine::{self, ItemSource, PlanOptions, ops};
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
    /// The events a hook can be written against, and when each fires. Sent
    /// rather than spelled out in the UI so the picker cannot offer an
    /// event the validator would then reject.
    pub hook_events: Vec<HookEvent>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    pub name: String,
    pub fires: String,
}

/// How one custom hook reaches one harness, for the editor's per-hook line.
/// Computed by `vstack_core::hook::delivery` — the same decision the engine
/// installs by — so the words on screen cannot drift from what applies.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HookDelivery {
    pub harness: HarnessId,
    pub mode: HookDeliveryMode,
    /// Why nothing installs, for `unavailable` rows.
    pub note: Option<String>,
}

#[derive(Serialize, Type, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum HookDeliveryMode {
    /// Registered in the harness's own hook configuration — it runs.
    Runs,
    /// Claude's per-agent hooks block — it runs, for the chosen agents.
    RunsInAgentFile,
    /// Written into the agents as instructions — nothing enforces it.
    Instructions,
    /// No surface for it here at all.
    Unavailable,
}

/// Per-hook, per-harness delivery for the hooks as currently drafted in the
/// editor — outer Vec in the order the hooks were passed.
#[tauri::command(async)]
#[specta::specta]
pub fn custom_hook_deliveries(
    scope: Scope,
    hooks: Vec<vstack_core::manifest::CustomHook>,
) -> Result<Vec<Vec<HookDelivery>>, String> {
    use vstack_core::hook::{Delivery, HookSpec, custom_hook_names, delivery};
    let env = env()?;
    let loaded = manifest::load_for_mutation(&manifest::manifest_path(&env, &scope))
        .map_err(|e| e.to_string())?;
    let mut draft = loaded.unwrap_or_default();
    draft.custom_hooks = hooks;
    let harnesses: Vec<HarnessId> = match draft.install.harnesses.is_empty() {
        true => HarnessId::ALL
            .into_iter()
            .filter(|h| vstack_core::harness::installable(*h))
            .collect(),
        false => draft.install.harnesses.clone(),
    };
    let names = custom_hook_names(&draft);
    Ok(draft
        .custom_hooks
        .iter()
        .zip(names)
        .map(|(hook, name)| {
            let spec = HookSpec::custom(hook, name);
            harnesses
                .iter()
                .filter(|harness| spec.applies_to(**harness))
                .map(|harness| {
                    let (mode, note) = match delivery(&env, &scope, *harness, &spec) {
                        Delivery::Registered => (HookDeliveryMode::Runs, None),
                        Delivery::InAgentFile => (HookDeliveryMode::RunsInAgentFile, None),
                        Delivery::Advisory => (HookDeliveryMode::Instructions, None),
                        Delivery::NotInstallable(reason) => {
                            (HookDeliveryMode::Unavailable, Some(reason))
                        }
                    };
                    HookDelivery {
                        harness: *harness,
                        mode,
                        note,
                    }
                })
                .collect()
        })
        .collect())
}

#[tauri::command(async)]
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
#[tauri::command(async)]
#[specta::specta]
pub fn update_manifest(scope: Scope, manifest: Manifest) -> Result<AuditView, String> {
    let env = env()?;
    let path = manifest::manifest_path(&env, &scope);
    let mut manifest = match manifest::load_for_mutation(&path).map_err(|e| e.to_string())? {
        Some(_) => manifest,
        None => on_first_creation(
            manifest,
            ops::manifest_for_mutation(&env, &scope).map_err(|e| e.to_string())?,
        ),
    };
    // A custom hook's name is its identity everywhere downstream; saving is
    // when a derived one stops being derived.
    vstack_core::hook::name_custom_hooks(&mut manifest);
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
                description: "Save vstack.toml".into(),
                op: Op::WriteManifest {
                    pre: Pre::observed(&path).map_err(|e| e.to_string())?,
                    path: path.clone(),
                    manifest: Box::new(manifest),
                },
            },
        );
    }
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn editor_inventory(scope: Scope) -> Result<EditorInventory, String> {
    let env = env()?;
    let loaded = manifest::load_for_mutation(&manifest::manifest_path(&env, &scope))
        .map_err(|e| e.to_string())?;
    let mut inventory = EditorInventory {
        declared_agents: Vec::new(),
        declared_skills: Vec::new(),
        available_skills: Vec::new(),
        // Per-harness settings are only offered for harnesses vstack
        // writes to.
        harnesses: HarnessId::ALL
            .into_iter()
            .filter(|h| vstack_core::harness::installable(*h))
            .collect(),
        hook_events: vstack_core::hook::EVENTS
            .iter()
            .map(|event| HookEvent {
                name: event.name.to_owned(),
                fires: event.fires.to_owned(),
            })
            .collect(),
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
        // A source that cannot be opened offers nothing; it must not take
        // the whole editor inventory down with it.
        let Ok(sealed) = vstack_core::source_read::SealedSource::open(&ready.root) else {
            continue;
        };
        let config = source::source_config(&sealed).map_err(|e| e.to_string())?;
        available.extend(source::list_items(&sealed, &config, ItemKind::Skill));
    }
    available.sort();
    available.dedup();
    inventory.available_skills = available;
    Ok(inventory)
}

/// The primary file behind one installed item, for the Library preview
/// pane — SKILL.md for a skill, the document itself for everything else
/// that has its own file.
#[tauri::command(async)]
#[specta::specta]
pub fn item_source(
    scope: Scope,
    kind: ItemKind,
    name: String,
    harness: HarnessId,
) -> Result<ItemSource, String> {
    engine::item_source(&env()?, &scope, kind, &name, harness).map_err(|e| e.to_string())
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
                    rev: None,
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
            name: None,
            event: "PreToolUse".to_owned(),
            matcher: Some("Bash".to_owned()),
            command: "./guard.sh".to_owned(),
            description: None,
            timeout: None,
            harnesses: None,
            enabled: true,
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
