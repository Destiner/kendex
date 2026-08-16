use std::fs;
use std::path::{Path, PathBuf};

use super::desired::native_dir;
use super::ops::manifest_for_mutation;
use crate::apply::{Op, Plan, PlannedOp, Pre};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::{self, ItemDecl, LOCAL_SOURCE_NAME};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::local_source_root;

/// Record an observed, unmanaged item into the manifest: its content moves
/// into the scope's local source (nothing is ever lost), the item is
/// declared from source `local`, and the original artifact goes to the
/// trash. A follow-up apply renders the managed replacement.
///
/// State machine: target-has-files → merge into declaration;
/// target-is-foreign-symlink → conflict, never clobber; broken symlink →
/// nothing to adopt, the follow-up apply recreates from declaration.
pub fn adopt(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    harness: HarnessId,
) -> Result<Plan> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let Some(dir) = native_dir(env, scope, harness, kind) else {
        return Err(CoreError::ItemNotInSource {
            name: name.to_owned(),
            source_name: format!("{} {}", harness.name(), kind.name()),
        });
    };
    let original = match kind {
        ItemKind::Agent => dir.join(crate::render::agent::file_name(harness, name)),
        _ => dir.join(name),
    };

    // Broken link: content is gone; declaring is all adoption can do. The
    // link itself is cleared by a planned op — planning never touches disk,
    // so a plan that is never applied (or fails) leaves the world as it was.
    let mut broken_link: Option<Pre> = None;
    if original.is_symlink() {
        let points_to = fs::read_link(&original).map_err(|e| CoreError::io(&original, e))?;
        if original.exists() {
            return Err(CoreError::ForeignSymlink {
                target: original,
                points_to,
            });
        }
        broken_link = Some(Pre::SymlinkTo { target: points_to });
    }

    let local_root = local_source_root(env, scope);
    let local_item = match kind {
        ItemKind::Skill => local_root.join("skills").join(name),
        ItemKind::Agent => local_root.join("agents").join(format!("{name}.md")),
        other => {
            return Err(CoreError::ItemNotInSource {
                name: name.to_owned(),
                source_name: format!("adopt does not support {} yet", other.name()),
            });
        }
    };
    let mut ops = capture_ops(kind, name, original, &local_item, broken_link)?;

    let harness_is_default = manifest.install.harnesses.contains(&harness);
    let decl = manifest
        .declared_mut(kind)
        .entry(name.to_owned())
        .or_insert_with(|| ItemDecl::from_source(LOCAL_SOURCE_NAME));
    decl.source = LOCAL_SOURCE_NAME.to_owned();
    if decl.harnesses.is_none() && !harness_is_default {
        decl.harnesses = Some(vec![harness]);
    }

    let manifest_path = manifest::manifest_path(env, scope);
    ops.push(PlannedOp {
        description: "declare the adopted item in vstack.toml".into(),
        op: Op::WriteManifest {
            pre: Pre::observed(&manifest_path)?,
            path: manifest_path,
            manifest: Box::new(manifest),
        },
    });
    Ok(Plan {
        scope: scope.clone(),
        ops,
    })
}

/// Move the observed artifact into the local source and clear what it left
/// behind. Nothing here runs at plan time: every byte read becomes an op.
fn capture_ops(
    kind: ItemKind,
    name: &str,
    original: PathBuf,
    local_item: &Path,
    broken_link: Option<Pre>,
) -> Result<Vec<PlannedOp>> {
    let mut ops = Vec::new();
    if let Some(pre) = broken_link {
        ops.push(PlannedOp {
            description: format!("clear the broken link at {}", original.display()),
            op: Op::Trash {
                path: original.clone(),
                pre,
            },
        });
    }
    if !original.exists() {
        if !local_item.exists() {
            return Err(CoreError::ItemNotInSource {
                name: name.to_owned(),
                source_name: format!("nothing at {} to adopt", original.display()),
            });
        }
        return Ok(ops);
    }
    // A copy the local source already holds is not overwritten in place:
    // it goes to the trash first, where it can be got back.
    if local_item.exists() {
        ops.push(PlannedOp {
            description: format!("trash the local source's earlier copy of {name}"),
            op: Op::Trash {
                path: local_item.to_path_buf(),
                pre: Pre::HashIs {
                    hash: crate::hash::hash_tree(local_item)?,
                },
            },
        });
    }
    let capture = match kind {
        ItemKind::Skill => Op::WriteTree {
            root: local_item.to_path_buf(),
            files: read_tree(&original)?,
            pre: Pre::Absent,
        },
        _ => Op::WriteFile {
            path: local_item.to_path_buf(),
            bytes: fs::read(&original).map_err(|e| CoreError::io(&original, e))?,
            pre: Pre::Absent,
        },
    };
    ops.push(PlannedOp {
        description: format!("move {name} into the local source"),
        op: capture,
    });
    ops.push(PlannedOp {
        description: format!("trash the unmanaged original at {}", original.display()),
        op: Op::Trash {
            path: original,
            pre: Pre::Any,
        },
    });
    Ok(ops)
}

pub(crate) fn read_tree(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    fn walk(dir: &Path, rel: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(|e| CoreError::io(dir, e))? {
            // A per-entry read error is not silently skipped: dropping it
            // would capture an incomplete tree and then trash the
            // original, losing content the caller asked to keep.
            let entry = entry.map_err(|e| CoreError::io(dir, e))?;
            let path = entry.path();
            let Some(name) = path.file_name() else {
                continue;
            };
            let rel = rel.join(name);
            // A link is not plain content: following it would read whatever
            // it points at into the capture under this tree's name. Rather
            // than silently drop it (and then trash the original), refuse —
            // nothing the user asked to keep is lost without a word.
            if path.is_symlink() {
                return Err(CoreError::ForeignSymlink {
                    points_to: fs::read_link(&path).unwrap_or_default(),
                    target: path,
                });
            }
            if path.is_dir() {
                walk(&path, &rel, files)?;
            } else {
                files.push((rel, fs::read(&path).map_err(|e| CoreError::io(&path, e))?));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(root, Path::new(""), &mut files)?;
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::audit;
    use crate::env::FakeOs;

    #[test]
    fn adopting_a_handmade_skill_moves_merges_and_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let project = tmp.path().join("app");
        let scope = Scope::Project {
            root: project.clone(),
        };
        fs::create_dir_all(project.join(".claude/skills/handmade")).unwrap();
        fs::write(
            project.join(".claude/skills/handmade/SKILL.md"),
            "---\nname: handmade\ndescription: mine\n---\nMy content.\n",
        )
        .unwrap();

        let plan = adopt(&env, &scope, ItemKind::Skill, "handmade", HarnessId::Claude).unwrap();
        crate::apply::execute(&env, &plan, None).unwrap();

        // Content lives in the local source; the original is trashed.
        assert!(
            project
                .join(".vstack-local/skills/handmade/SKILL.md")
                .is_file()
        );
        assert!(!project.join(".claude/skills/handmade").exists());

        // Follow-up apply renders the managed replacement, drift-clean.
        let report = audit(&env, &scope).unwrap();
        crate::apply::execute(&env, &report.plan, None).unwrap();
        let link = project.join(".claude/skills/handmade");
        assert!(link.is_symlink());
        let rendered =
            fs::read_to_string(project.join(".agents/skills/handmade/SKILL.md")).unwrap();
        assert!(rendered.contains("My content."));
        let after = audit(&env, &scope).unwrap();
        assert_eq!(after.drift, vec![]);
    }

    /// The local source already had a copy: it is trashed, never overwritten
    /// in place, so nothing adoption replaces is gone for good.
    #[test]
    fn an_earlier_local_copy_goes_to_the_trash_not_under_the_new_one() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let project = tmp.path().join("app");
        let scope = Scope::Project {
            root: project.clone(),
        };
        let earlier = project.join(".vstack-local/skills/handmade");
        fs::create_dir_all(&earlier).unwrap();
        fs::write(earlier.join("SKILL.md"), "earlier").unwrap();
        fs::write(earlier.join("notes.md"), "kept only here").unwrap();
        fs::create_dir_all(project.join(".claude/skills/handmade")).unwrap();
        fs::write(project.join(".claude/skills/handmade/SKILL.md"), "observed").unwrap();

        let plan = adopt(&env, &scope, ItemKind::Skill, "handmade", HarnessId::Claude).unwrap();
        crate::apply::execute(&env, &plan, None).unwrap();

        assert_eq!(
            fs::read_to_string(earlier.join("SKILL.md")).unwrap(),
            "observed"
        );
        assert!(!earlier.join("notes.md").exists());
        let trashed: Vec<_> = fs::read_dir(env.trash_dir()).unwrap().flatten().collect();
        assert!(trashed.iter().any(|e| e.path().join("notes.md").is_file()));
    }

    #[test]
    fn foreign_symlinks_are_conflicts_never_clobbered() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fake(tmp.path(), FakeOs::Linux);
        let project = tmp.path().join("app");
        let scope = Scope::Project {
            root: project.clone(),
        };
        let elsewhere = tmp.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        fs::create_dir_all(project.join(".claude/skills")).unwrap();
        std::os::unix::fs::symlink(&elsewhere, project.join(".claude/skills/linked")).unwrap();

        let error = adopt(&env, &scope, ItemKind::Skill, "linked", HarnessId::Claude).unwrap_err();
        assert!(matches!(error, CoreError::ForeignSymlink { .. }));
        assert!(project.join(".claude/skills/linked").is_symlink());
    }
}
