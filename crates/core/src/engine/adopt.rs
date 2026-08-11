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

    if original.is_symlink() {
        let points_to = fs::read_link(&original).map_err(|e| CoreError::io(&original, e))?;
        if original.exists() {
            return Err(CoreError::ForeignSymlink {
                target: original,
                points_to,
            });
        }
        // Broken link: content is gone; declaring is all adoption can do.
        fs::remove_file(&original).map_err(|e| CoreError::io(&original, e))?;
    }

    let local_root = local_source_root(env, scope);
    let mut ops = Vec::new();
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

    if original.exists() {
        let files = read_tree(&original)?;
        match kind {
            ItemKind::Skill => ops.push(PlannedOp {
                description: format!("move {} into the local source", name),
                op: Op::WriteTree {
                    root: local_item.clone(),
                    files,
                    pre: Pre::Any,
                },
            }),
            ItemKind::Agent => {
                let bytes = fs::read(&original).map_err(|e| CoreError::io(&original, e))?;
                ops.push(PlannedOp {
                    description: format!("move {} into the local source", name),
                    op: Op::WriteFile {
                        path: local_item.clone(),
                        bytes,
                        pre: Pre::Any,
                    },
                });
            }
            _ => {}
        }
        ops.push(PlannedOp {
            description: format!("trash the unmanaged original at {}", original.display()),
            op: Op::Trash {
                path: original,
                pre: Pre::Any,
            },
        });
    } else if !local_item.exists() {
        return Err(CoreError::ItemNotInSource {
            name: name.to_owned(),
            source_name: format!("nothing at {} to adopt", original.display()),
        });
    }

    let harness_is_default = manifest.install.harnesses.contains(&harness);
    let decl = manifest
        .declared_mut(kind)
        .entry(name.to_owned())
        .or_insert_with(|| ItemDecl::from_source(LOCAL_SOURCE_NAME));
    decl.source = LOCAL_SOURCE_NAME.to_owned();
    if decl.harnesses.is_none() && !harness_is_default {
        decl.harnesses = Some(vec![harness]);
    }

    ops.push(PlannedOp {
        description: "declare the adopted item in vstack.toml".into(),
        op: Op::WriteManifest {
            path: manifest::manifest_path(env, scope),
            manifest: Box::new(manifest),
        },
    });
    Ok(Plan {
        scope: scope.clone(),
        ops,
    })
}

fn read_tree(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    fn walk(dir: &Path, rel: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) -> Result<()> {
        for entry in fs::read_dir(dir)
            .map_err(|e| CoreError::io(dir, e))?
            .flatten()
        {
            let path = entry.path();
            let Some(name) = path.file_name() else {
                continue;
            };
            let rel = rel.join(name);
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
