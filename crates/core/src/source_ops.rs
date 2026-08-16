use serde::Serialize;
use specta::Type;

use crate::engine::{EngineReport, PlanOptions, plan_scope};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::lock::{load as load_lock, lock_path};
use crate::manifest::{self, Manifest, SourceDecl};
use crate::model::Scope;

/// Everything the Sources page shows for one declared source in one scope.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceRow {
    pub scope: Scope,
    pub name: String,
    pub reference: String,
    pub is_remote: bool,
    pub enabled: bool,
    /// Cache HEAD for remotes — freshness display.
    pub head: Option<String>,
    pub declared_items: Vec<String>,
}

fn load_current(env: &Env, scope: &Scope) -> Result<Option<Manifest>> {
    match manifest::load(&manifest::manifest_path(env, scope))? {
        manifest::ManifestFile::Current(m) => Ok(Some(*m)),
        _ => Ok(None),
    }
}

fn referents(manifest: &Manifest, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (table, items) in [
        ("agents", &manifest.agents),
        ("skills", &manifest.skills),
        ("hooks", &manifest.hooks),
        ("commands", &manifest.commands),
        ("mcp-servers", &manifest.mcp_servers),
        ("pi-extensions", &manifest.pi_extensions),
        ("bundles", &manifest.bundles),
    ] {
        for (name, decl) in items {
            if decl.source == source {
                names.push(format!("{table}.{name}"));
            }
        }
    }
    names
}

pub fn list_sources(env: &Env, scope: &Scope) -> Result<Vec<SourceRow>> {
    let Some(manifest) = load_current(env, scope)? else {
        return Ok(Vec::new());
    };
    Ok(manifest
        .sources
        .iter()
        .map(|(name, decl)| SourceRow {
            scope: scope.clone(),
            name: name.clone(),
            reference: decl
                .repo
                .clone()
                .or_else(|| decl.path.clone())
                .unwrap_or_default(),
            is_remote: decl.repo.is_some(),
            enabled: decl.enabled,
            head: decl
                .repo
                .as_deref()
                .and_then(|repo| crate::remote::cache_head(env, repo, decl.rev.as_deref())),
            declared_items: referents(&manifest, name),
        })
        .collect())
}

/// One curated set a catalog offers, as the Catalogs page lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BundleRow {
    pub scope: Scope,
    pub source: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub category: Option<String>,
    /// What it carries, each as the kind and name it installs under.
    pub members: Vec<String>,
    pub installed: bool,
}

/// Every set the readable catalogs in this scope offer. A catalog that cannot
/// be read right now offers nothing here — the Catalogs page already says why
/// that source is unreachable, and repeating it per set would say it twice.
pub fn list_bundles(env: &Env, scope: &Scope) -> Result<Vec<BundleRow>> {
    let Some(manifest) = load_current(env, scope)? else {
        return Ok(Vec::new());
    };
    let mut rows = Vec::new();
    for source in manifest.sources.keys() {
        let Ok(crate::source::SourceState::Ready(ready)) =
            crate::source::resolve(env, scope, source, &manifest)
        else {
            continue;
        };
        let Ok(sealed) = crate::source_read::SealedSource::open(&ready.root) else {
            continue;
        };
        let Ok(config) = crate::source::source_config(&sealed) else {
            continue;
        };
        let Ok(offered) = crate::source::bundles::offered(&sealed, &config) else {
            continue;
        };
        for bundle in offered {
            rows.push(BundleRow {
                scope: scope.clone(),
                source: source.clone(),
                installed: manifest
                    .bundles
                    .get(&bundle.name)
                    .is_some_and(|decl| &decl.source == source),
                name: bundle.name,
                description: bundle.description,
                version: bundle.version,
                category: bundle.category,
                members: bundle
                    .members
                    .into_iter()
                    .map(|member| format!("{} {}", member.kind.name(), member.name))
                    .collect(),
            });
        }
    }
    Ok(rows)
}

pub(crate) fn persist_and_plan(
    env: &Env,
    scope: &Scope,
    manifest: Manifest,
) -> Result<EngineReport> {
    let lock = load_lock(&lock_path(env, scope))?;
    let mut report = plan_scope(env, scope, &manifest, &lock, &PlanOptions::default())?;
    let has_write = report
        .plan
        .ops
        .iter()
        .any(|op| matches!(op.op, crate::apply::Op::WriteManifest { .. }));
    if !has_write {
        let path = manifest::manifest_path(env, scope);
        report.plan.ops.insert(
            0,
            crate::apply::PlannedOp {
                description: "Save vstack.toml".into(),
                op: crate::apply::Op::WriteManifest {
                    pre: crate::apply::Pre::observed(&path)?,
                    path,
                    manifest: Box::new(manifest),
                },
            },
        );
    }
    Ok(report)
}

/// Declare a source. `owner/repo`-shaped references become remotes,
/// anything else a path. `owner/repo@<rev>` pins or tracks a revision: a
/// commit id is a pin, a tag or branch is followed.
pub fn add_source(env: &Env, scope: &Scope, name: &str, reference: &str) -> Result<EngineReport> {
    let mut manifest = crate::engine::ops::manifest_for_mutation(env, scope)?;
    let (repo, rev) = match reference.split_once('@') {
        Some((repo, rev)) if !repo.is_empty() && !rev.is_empty() => (repo, Some(rev)),
        _ => (reference, None),
    };
    let is_repo = repo.contains('/')
        && !repo.starts_with('.')
        && !repo.starts_with('/')
        && !repo.starts_with('~')
        && repo.matches('/').count() == 1;
    let decl = if is_repo {
        SourceDecl {
            repo: Some(repo.to_owned()),
            path: None,
            rev: rev.map(str::to_owned),
            enabled: true,
        }
    } else {
        SourceDecl {
            repo: None,
            path: Some(reference.to_owned()),
            rev: None,
            enabled: true,
        }
    };
    manifest.sources.insert(name.to_owned(), decl);
    persist_and_plan(env, scope, manifest)
}

/// Removal is blocked while declarations still reference the source — the
/// error names every referent and the fix; disable stays available.
pub fn remove_source(env: &Env, scope: &Scope, name: &str) -> Result<EngineReport> {
    let mut manifest = crate::engine::ops::manifest_for_mutation(env, scope)?;
    if !manifest.sources.contains_key(name) {
        return Err(CoreError::UnknownSource {
            name: name.to_owned(),
        });
    }
    let used_by = referents(&manifest, name);
    if !used_by.is_empty() {
        return Err(CoreError::ManifestInvalid {
            path: manifest::manifest_path(env, scope),
            findings: vec![format!(
                "sources.{name}: still referenced by {} — fix: remove those items first, or disable the source instead",
                used_by.join(", ")
            )],
        });
    }
    manifest.sources.remove(name);
    persist_and_plan(env, scope, manifest)
}

/// Disabling deactivates the source's installations in place; re-enabling
/// restores them (they stay declared throughout — not drift).
pub fn toggle_source(env: &Env, scope: &Scope, name: &str, enabled: bool) -> Result<EngineReport> {
    let mut manifest = crate::engine::ops::manifest_for_mutation(env, scope)?;
    let Some(decl) = manifest.sources.get_mut(name) else {
        return Err(CoreError::UnknownSource {
            name: name.to_owned(),
        });
    };
    decl.enabled = enabled;
    persist_and_plan(env, scope, manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, Env, Scope) {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let project = tmp.path().join("app");
        let source = tmp.path().join("catalog");
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::create_dir_all(source.join("skills/gh")).unwrap();
        fs::write(source.join("skills/gh/SKILL.md"), "---\nname: gh\n---\nx\n").unwrap();
        fs::write(
            project.join("vstack.toml"),
            format!(
                "schema = 3\n[sources.cat]\npath = \"{}\"\n[install]\nharnesses = [\"claude\"]\n[skills.gh]\nsource = \"cat\"\n",
                source.display()
            ),
        )
        .unwrap();
        let scope = Scope::Project { root: project };
        (tmp, env, scope)
    }

    /// A reference can name the revision to read. A path that happens to
    /// contain an `@` is still a path — the split only counts where what
    /// precedes it is repository-shaped.
    #[test]
    fn a_reference_can_carry_the_revision_it_reads() {
        let (_tmp, env, scope) = fixture();
        for (name, reference) in [
            ("pinned", "owner/repo@v1.2.0"),
            ("tracked", "owner/repo"),
            ("here", "../my@catalog"),
        ] {
            let report = add_source(&env, &scope, name, reference).unwrap();
            crate::apply::execute(&env, &report.plan, None).unwrap();
        }
        let manifest = load_current(&env, &scope).unwrap().unwrap();

        let pinned = &manifest.sources["pinned"];
        assert_eq!(pinned.repo.as_deref(), Some("owner/repo"));
        assert_eq!(pinned.rev.as_deref(), Some("v1.2.0"));
        assert_eq!(manifest.sources["tracked"].rev, None);
        let here = &manifest.sources["here"];
        assert_eq!(here.path.as_deref(), Some("../my@catalog"));
        assert_eq!(here.rev, None);
    }

    #[test]
    fn removal_is_blocked_while_referenced_then_allowed() {
        let (_tmp, env, scope) = fixture();
        let error = remove_source(&env, &scope, "cat").unwrap_err();
        assert!(error.to_string().contains("skills.gh"));
        assert!(error.to_string().contains("disable the source"));

        let report = crate::engine::ops::remove(&env, &scope, &["gh".to_owned()], false).unwrap();
        crate::apply::execute(&env, &report.plan, None).unwrap();
        let report = remove_source(&env, &scope, "cat").unwrap();
        crate::apply::execute(&env, &report.plan, None).unwrap();
        assert!(list_sources(&env, &scope).unwrap().is_empty());
    }

    #[test]
    fn disable_deactivates_without_drift_and_reenable_restores() {
        let (_tmp, env, scope) = fixture();
        let report = crate::engine::audit(&env, &scope).unwrap();
        crate::apply::execute(&env, &report.plan, None).unwrap();
        let link = match &scope {
            Scope::Project { root } => root.join(".claude/skills/gh"),
            Scope::Global => unreachable!("fixture scope is a project"),
        };
        assert!(link.is_symlink());

        let report = toggle_source(&env, &scope, "cat", false).unwrap();
        crate::apply::execute(&env, &report.plan, None).unwrap();
        // Declared but inactive: the artifact stays (nothing to render it
        // away against), and audit reports no drift for it.
        let after = crate::engine::audit(&env, &scope).unwrap();
        assert!(after.notes.iter().any(|n| n.contains("disabled")));
        assert!(!after.drift.iter().any(|r| r.name == "gh"));

        let report = toggle_source(&env, &scope, "cat", true).unwrap();
        crate::apply::execute(&env, &report.plan, None).unwrap();
        let clean = crate::engine::audit(&env, &scope).unwrap();
        assert_eq!(clean.drift, vec![]);
        assert!(link.is_symlink());
    }

    #[test]
    fn rows_show_reference_enablement_and_referents() {
        let (_tmp, env, scope) = fixture();
        let rows = list_sources(&env, &scope).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "cat");
        assert!(!rows[0].is_remote);
        assert!(rows[0].enabled);
        assert_eq!(rows[0].declared_items, ["skills.gh"]);
    }
}
