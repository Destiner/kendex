use std::collections::BTreeSet;
use std::path::PathBuf;

use super::{DriftRow, DriftState};
use crate::apply::{Op, PlannedOp, Pre};
use crate::error::Result;
use crate::hash::hash_tree;
use crate::lock::{Lock, LockEntry, timestamp};
use crate::model::Scope;

use super::desired::{Artifact, Desired, artifact_disk_hash};

pub(super) fn plan_item(
    item: &Desired,
    scope: &Scope,
    lock: &Lock,
    drift: &mut Vec<DriftRow>,
    ops: &mut Vec<PlannedOp>,
    new_lock: &mut Lock,
    written_canonicals: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let row = |state: DriftState, detail: String| DriftRow {
        kind: item.kind,
        name: item.name.clone(),
        harness: item.harness,
        scope: scope.clone(),
        state,
        detail,
    };
    let existing = lock.entries.get(&item.key);

    // Invariant 4: a recorded source is never silently rebound.
    if let Some(entry) = existing
        && entry.source_repo != item.provenance
        && entry.source_repo != "local"
    {
        drift.push(row(
            DriftState::Conflict,
            format!(
                "installed from {} but now declared from {} — remove it first",
                entry.source_repo, item.provenance
            ),
        ));
        new_lock.entries.insert(item.key.clone(), entry.clone());
        return Ok(());
    }

    let planned = match &item.artifact {
        Artifact::File { .. } => plan_file(item, existing.is_some(), ops),
        Artifact::Tree { .. } => plan_tree(item, existing.is_some(), written_canonicals, ops),
        Artifact::Registration { .. } => plan_registration(item, existing.is_some(), ops),
    }?;
    let dirty = !matches!(planned, Planned::Clean);
    match planned {
        Planned::Conflict(detail) => {
            drift.push(row(DriftState::Conflict, detail));
            if let Some(entry) = existing {
                new_lock.entries.insert(item.key.clone(), entry.clone());
            }
            return Ok(());
        }
        Planned::Drift(state, detail) => drift.push(row(state, detail)),
        Planned::Clean => {}
    }

    let hash_moved = existing.is_some_and(|entry| entry.source_hash != item.hash);
    if hash_moved && !dirty {
        drift.push(row(
            DriftState::Stale,
            "source or customization changed since install".into(),
        ));
    }
    let installed_at = match existing {
        Some(entry) if !dirty && !hash_moved => entry.installed_at.clone(),
        _ => timestamp(),
    };
    new_lock.entries.insert(
        item.key.clone(),
        LockEntry {
            name: item.name.clone(),
            kind: item.kind,
            harness: item.harness,
            source: item.source_name.clone(),
            source_repo: item.provenance.clone(),
            method: item.method,
            installed_at,
            source_hash: item.hash.clone(),
            enabled: item.enabled,
            upstream_skills: item.upstream_skills.clone(),
        },
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum Planned {
    Clean,
    Drift(DriftState, String),
    Conflict(String),
}

fn plan_file(item: &Desired, locked: bool, ops: &mut Vec<PlannedOp>) -> Result<Planned> {
    let Artifact::File { path, bytes } = &item.artifact else {
        return Ok(Planned::Clean);
    };
    plan_written_file(item, path, bytes, locked, ops)
}

fn plan_written_file(
    item: &Desired,
    path: &std::path::Path,
    bytes: &[u8],
    locked: bool,
    ops: &mut Vec<PlannedOp>,
) -> Result<Planned> {
    if path.is_symlink() {
        return Ok(Planned::Conflict(format!(
            "{} is a foreign symlink",
            path.display()
        )));
    }
    if path.exists() && !path.is_file() {
        return Ok(Planned::Conflict(format!(
            "a directory sits at {}",
            path.display()
        )));
    }
    let disk = path.is_file().then(|| hash_tree(path)).transpose()?;
    let wanted = crate::hash::hash_bytes(bytes);
    match disk {
        Some(current) if current == wanted => Ok(Planned::Clean),
        Some(current) => {
            if !locked {
                return Ok(Planned::Conflict(format!(
                    "unmanaged file at {} — adopt it first",
                    path.display()
                )));
            }
            ops.push(PlannedOp {
                description: format!("rewrite {}", path.display()),
                op: Op::WriteFile {
                    path: path.to_path_buf(),
                    bytes: bytes.to_vec(),
                    pre: Pre::HashIs { hash: current },
                },
            });
            Ok(Planned::Drift(DriftState::Stale, "content changed".into()))
        }
        None => Ok(plan_absent_file(item, path, bytes, locked, ops)),
    }
}

/// Nothing at the target: our own content may be waiting under the toggled
/// name, otherwise this is a fresh install. Anything else occupying the
/// toggled name belongs to someone else and is never written through.
fn plan_absent_file(
    item: &Desired,
    path: &std::path::Path,
    bytes: &[u8],
    locked: bool,
    ops: &mut Vec<PlannedOp>,
) -> Planned {
    let alternate = toggle_sibling(path);
    if alternate.is_symlink() {
        return Planned::Conflict(format!("foreign symlink at {}", alternate.display()));
    }
    if alternate.is_file() {
        if !locked {
            return Planned::Conflict(format!(
                "unmanaged file at {} — adopt it first",
                alternate.display()
            ));
        }
        ops.push(PlannedOp {
            description: format!(
                "{} {}",
                if item.enabled { "enable" } else { "disable" },
                item.name
            ),
            op: Op::Rename {
                from: alternate,
                to: path.to_path_buf(),
                to_pre: Pre::Absent,
            },
        });
        ops.push(PlannedOp {
            description: format!("refresh {}", path.display()),
            op: Op::WriteFile {
                path: path.to_path_buf(),
                bytes: bytes.to_vec(),
                pre: Pre::Any,
            },
        });
        let wanted_state = if item.enabled { "enabled" } else { "disabled" };
        return Planned::Drift(DriftState::Stale, format!("declared {wanted_state}"));
    }
    ops.push(PlannedOp {
        description: format!("write {}", path.display()),
        op: Op::WriteFile {
            path: path.to_path_buf(),
            bytes: bytes.to_vec(),
            pre: Pre::Absent,
        },
    });
    Planned::Drift(DriftState::Missing, "not on disk".into())
}

fn plan_tree(
    item: &Desired,
    locked: bool,
    written_canonicals: &mut BTreeSet<PathBuf>,
    ops: &mut Vec<PlannedOp>,
) -> Result<Planned> {
    let Artifact::Tree {
        canonical,
        files,
        link,
    } = &item.artifact
    else {
        return Ok(Planned::Clean);
    };
    if canonical.is_symlink() {
        return Ok(Planned::Conflict(format!(
            "{} is a foreign symlink",
            canonical.display()
        )));
    }
    if canonical.exists() && !canonical.is_dir() {
        return Ok(Planned::Conflict(format!(
            "a file sits at {}",
            canonical.display()
        )));
    }
    let wanted = artifact_disk_hash(&item.artifact);
    let disk = canonical
        .is_dir()
        .then(|| hash_tree(canonical))
        .transpose()?;
    let mut result = Planned::Clean;
    if disk.as_deref() != Some(wanted.as_str()) {
        if disk.is_some() && !locked && !written_canonicals.contains(canonical) {
            return Ok(Planned::Conflict(format!(
                "unmanaged content at {} — adopt it first",
                canonical.display()
            )));
        }
        result = match disk {
            Some(_) => Planned::Drift(DriftState::Stale, "content changed".into()),
            None => Planned::Drift(DriftState::Missing, "not on disk".into()),
        };
        if written_canonicals.insert(canonical.clone()) {
            ops.push(PlannedOp {
                description: format!("render {}", canonical.display()),
                op: Op::WriteTree {
                    root: canonical.clone(),
                    files: files.clone(),
                    pre: match disk {
                        Some(hash) => Pre::HashIs { hash },
                        None => Pre::Absent,
                    },
                },
            });
        }
    }
    let Some(link) = link else {
        return Ok(result);
    };
    if link.is_symlink() {
        let points_to = std::fs::read_link(link).unwrap_or_default();
        if &points_to != canonical {
            return Ok(Planned::Conflict(format!(
                "{} links elsewhere ({})",
                link.display(),
                points_to.display()
            )));
        }
        Ok(result)
    } else if link.exists() {
        Ok(Planned::Conflict(format!(
            "{} occupied by unmanaged content — adopt it first",
            link.display()
        )))
    } else {
        ops.push(PlannedOp {
            description: format!("link {} → {}", link.display(), canonical.display()),
            op: Op::Symlink {
                link: link.clone(),
                target: canonical.clone(),
                pre: Pre::Absent,
            },
        });
        Ok(Planned::Drift(
            DriftState::Missing,
            format!("no link at {}", link.display()),
        ))
    }
}

/// A registration is in sync when its backing file matches and re-applying
/// every config edit changes nothing. That idempotency is the whole drift
/// check — unrelated keys in those shared files are never read as ours.
fn plan_registration(item: &Desired, locked: bool, ops: &mut Vec<PlannedOp>) -> Result<Planned> {
    let Artifact::Registration { script, edits } = &item.artifact else {
        return Ok(Planned::Clean);
    };
    let mut planned = match script {
        Some((path, bytes)) => plan_written_file(item, path, bytes, locked, ops)?,
        None => Planned::Clean,
    };
    if matches!(planned, Planned::Conflict(_)) {
        return Ok(planned);
    }
    for (path, edit) in edits {
        let current = crate::fs::read_if_exists(path)?.unwrap_or_default();
        let updated =
            edit.apply(&current)
                .map_err(|message| crate::error::CoreError::ConfigEdit {
                    path: path.clone(),
                    message,
                })?;
        if updated == current {
            continue;
        }
        ops.push(PlannedOp {
            description: format!("register {} in {}", item.name, path.display()),
            op: Op::EditFile {
                path: path.clone(),
                edit: edit.clone(),
                pre: match path.is_file() {
                    true => Pre::HashIs {
                        hash: hash_tree(path)?,
                    },
                    false => Pre::Absent,
                },
            },
        });
        if matches!(planned, Planned::Clean) {
            planned = match locked {
                true => Planned::Drift(DriftState::Stale, "registration out of sync".into()),
                false => Planned::Drift(DriftState::Missing, "not registered".into()),
            };
        }
    }
    Ok(planned)
}

/// A declared-disabled artifact keeps its content under the `.disabled`
/// name; toggling is a rename.
fn toggle_sibling(path: &std::path::Path) -> PathBuf {
    let text = path.display().to_string();
    match text.strip_suffix(".disabled") {
        Some(base) => PathBuf::from(base),
        None => PathBuf::from(format!("{text}.disabled")),
    }
}
