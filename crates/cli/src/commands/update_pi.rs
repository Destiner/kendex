use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vstack_core::env::Env;
use vstack_core::harness::HarnessAdapter;
use vstack_core::harness::pi::Pi;
use vstack_core::manifest::ManifestFile;
use vstack_core::model::Scope;
use vstack_core::{manifest, pi_ext, settings, source};

use super::{CliResult, out, resolve_scopes, say};
use crate::scope::ScopeFilter;

/// What update-pi found for one installed package.
enum Status {
    Current,
    /// The declared source ships different bytes than the installed copy.
    Stale {
        source_dir: PathBuf,
    },
    /// Installed under `packages/`, but no declared source ships it.
    Unsourced,
    /// An `npm:` entry in Pi's settings: Pi resolves these itself, so vstack
    /// reports the version and leaves the package alone.
    Npm {
        latest: Option<String>,
    },
}

struct Row {
    name: String,
    version: Option<String>,
    status: Status,
}

struct ScopePlan {
    label: String,
    root: PathBuf,
    rows: Vec<Row>,
    notes: Vec<String>,
}

/// Compare every installed Pi package against the source it came from and
/// reinstall the ones that fell behind.
pub fn run(env: &Env, filter: ScopeFilter, check: bool) -> CliResult {
    let roots = settings::load(env)?.harness_roots;
    let mut plans = Vec::new();
    for scope in resolve_scopes(env, filter)? {
        let root = match &scope {
            Scope::Global => roots
                .get(Pi.id().name())
                .cloned()
                .unwrap_or_else(|| Pi.default_global_root(env)),
            Scope::Project { root } => root.join(".pi"),
        };
        if root.is_dir() {
            plans.push(plan_scope(env, &scope, root)?);
        }
    }

    if plans.is_empty() {
        say("no pi scope on this machine");
        return Ok(());
    }
    for plan in &plans {
        print_plan(plan);
    }

    if check {
        let stale = plans.iter().flat_map(|p| &p.rows).filter(is_stale).count();
        if stale > 0 {
            say(&format!(
                "{stale} package(s) can be updated — run without --check to apply"
            ));
        }
        return Ok(());
    }
    update(env, &plans)
}

fn is_stale(row: &&Row) -> bool {
    matches!(row.status, Status::Stale { .. })
}

fn plan_scope(
    env: &Env,
    scope: &Scope,
    root: PathBuf,
) -> Result<ScopePlan, Box<dyn std::error::Error>> {
    let mut notes = Vec::new();
    let sources = declared_sources(env, scope, &mut notes);
    let mut rows = Vec::new();

    for name in pi_ext::list_installed(&root)? {
        let status = match sources.get(&name) {
            None => Status::Unsourced,
            Some(source_dir) => {
                let installed = pi_ext::installed_hash(&root, &name)?;
                if installed.is_some() && installed == pi_ext::package_hash(source_dir)? {
                    Status::Current
                } else {
                    Status::Stale {
                        source_dir: source_dir.clone(),
                    }
                }
            }
        };
        let version = installed_version(&root, &name);
        rows.push(Row {
            name,
            version,
            status,
        });
    }

    for name in pi_ext::list_npm_entries(&root)? {
        let version = installed_version(&root, &name);
        let latest = npm_latest(&name);
        rows.push(Row {
            name,
            version,
            status: Status::Npm { latest },
        });
    }

    Ok(ScopePlan {
        label: scope.label(),
        root,
        rows,
        notes,
    })
}

/// Where each declared pi-extension's bytes live right now. A source that
/// cannot be read is a note, not a failure — the rest of the scope still
/// updates.
fn declared_sources(
    env: &Env,
    scope: &Scope,
    notes: &mut Vec<String>,
) -> BTreeMap<String, PathBuf> {
    let mut found = BTreeMap::new();
    let path = manifest::manifest_path(env, scope);
    let manifest = match manifest::load(&path) {
        Ok(ManifestFile::Current(manifest)) => manifest,
        Ok(_) => return found,
        Err(error) => {
            notes.push(error.to_string());
            return found;
        }
    };
    for (name, decl) in &manifest.pi_extensions {
        match source::require_ready(env, scope, &decl.source, &manifest) {
            Ok(ready) => {
                let dir = ready.root.join("pi-extensions").join(name);
                if dir.join("package.json").is_file() {
                    found.insert(name.clone(), dir);
                } else {
                    notes.push(format!(
                        "{name}: source '{}' no longer ships pi-extensions/{name}",
                        decl.source
                    ));
                }
            }
            Err(error) => notes.push(format!("{name}: {error}")),
        }
    }
    found
}

fn installed_version(root: &Path, name: &str) -> Option<String> {
    pi_ext::read(&pi_ext::packages_dir(root).join(name))
        .ok()
        .and_then(|package| package.version)
}

/// Best effort: no npm, no network, or an unpublished package all read as an
/// unknown latest version rather than a failed run.
fn npm_latest(name: &str) -> Option<String> {
    let output = Command::new("npm")
        .args(["view", name, "version", "--json"])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    serde_json::from_str::<serde_json::Value>(text.trim())
        .ok()?
        .as_str()
        .map(str::to_owned)
}

fn semver(version: &str) -> Vec<u64> {
    let mut parts: Vec<u64> = version
        .trim()
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse().unwrap_or_default())
        .collect();
    parts.resize(3, 0);
    parts
}

fn print_plan(plan: &ScopePlan) {
    say(&format!("{} ({})", plan.label, plan.root.display()));
    if plan.rows.is_empty() {
        say("  no pi packages installed");
    }
    for row in &plan.rows {
        out(&format!(
            "  {:<34} {:<22} {}",
            row.name,
            versions(row),
            describe(row)
        ));
    }
    for note in &plan.notes {
        say(&format!("  ! {note}"));
    }
}

fn versions(row: &Row) -> String {
    let installed = row.version.as_deref().unwrap_or("-");
    match &row.status {
        Status::Npm {
            latest: Some(latest),
        } if latest != installed => {
            format!("{installed} -> {latest}")
        }
        _ => installed.to_owned(),
    }
}

fn describe(row: &Row) -> String {
    match &row.status {
        Status::Current => "up to date".to_owned(),
        Status::Stale { .. } => "stale (source changed)".to_owned(),
        Status::Unsourced => "no declared source".to_owned(),
        Status::Npm { latest } => match latest {
            None => "npm, latest unknown".to_owned(),
            Some(latest) => match &row.version {
                Some(installed) if semver(latest) > semver(installed) => {
                    "npm, update available".to_owned()
                }
                Some(_) => "npm, up to date".to_owned(),
                None => "npm, managed by pi".to_owned(),
            },
        },
    }
}

fn update(env: &Env, plans: &[ScopePlan]) -> CliResult {
    let mut updated = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for plan in plans {
        for row in &plan.rows {
            let Status::Stale { source_dir } = &row.status else {
                continue;
            };
            match pi_ext::install(env, &plan.root, source_dir) {
                Ok(outcome) => {
                    updated += 1;
                    out(&format!(
                        "  updated {} -> {}",
                        row.name,
                        outcome.version.as_deref().unwrap_or("?")
                    ));
                    for bin in &outcome.unbuilt_bins {
                        say(&format!(
                            "  ! {}: bin '{bin}' is not built, so no command was linked",
                            row.name
                        ));
                    }
                }
                Err(error) => {
                    say(&format!("  failed {}: {error}", row.name));
                    failures.push(format!("{} ({})", row.name, plan.label));
                }
            }
        }
    }
    if failures.is_empty() {
        say(&match updated {
            0 => "all pi packages up to date".to_owned(),
            count => format!("updated {count} package(s)"),
        });
        return Ok(());
    }
    Err(format!("update failed for: {}", failures.join(", ")).into())
}
