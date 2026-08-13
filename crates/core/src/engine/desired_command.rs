use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::error::Result;
use crate::lock::EmittedArtifact;
use crate::model::{HarnessId, ItemKind};
use crate::render::agent::GENERATED_BANNER;
use crate::render::yaml_scalar;

use super::ItemWarning;
use super::desired::{Artifact, Desired, DesiredState, ItemCtx, native_dir, target_harnesses};
use super::desired_kinds::declared;
use super::targets::disabled_name;

pub(super) fn desired_command(ctx: &ItemCtx, state: &mut DesiredState) -> Result<()> {
    let bytes = ctx.sealed.read(ctx.item_path)?;
    for harness in ctx.harnesses.clone() {
        let item = match crate::harness::capabilities(harness, ItemKind::Command).installs_as {
            None => native_file(ctx, harness, &bytes)?,
            Some(ItemKind::Skill) => as_skill(ctx, state, harness, &bytes)?,
            Some(kind) => {
                state.notes.push(format!(
                    "command {}: {} stores commands as {}s, which vstack cannot write",
                    ctx.name,
                    harness.display_name(),
                    kind.name()
                ));
                None
            }
        };
        state.items.extend(item);
    }
    Ok(())
}

/// The harness reads commands from a directory of its own: one file, named
/// for the command, parked under `.disabled` while it is turned off.
fn native_file(ctx: &ItemCtx, harness: HarnessId, bytes: &[u8]) -> Result<Option<Desired>> {
    let Some(dir) = native_dir(ctx.env, ctx.scope, harness, ItemKind::Command) else {
        return Ok(None);
    };
    let file = dir.join(format!("{}.md", ctx.name));
    let artifact = Artifact::File {
        path: match ctx.decl.enabled {
            true => file,
            false => disabled_name(&file),
        },
        bytes: bytes.to_vec(),
    };
    Ok(Some(declared(ctx, ItemKind::Command, harness, artifact)?))
}

/// Codex has no command directory to write to — it retired prompts in favor
/// of skills — so the command becomes a one-file skill tree on the skill
/// surface, and the lock carries the name and path it took.
fn as_skill(
    ctx: &ItemCtx,
    state: &mut DesiredState,
    harness: HarnessId,
    bytes: &[u8],
) -> Result<Option<Desired>> {
    let Some(dir) = native_dir(ctx.env, ctx.scope, harness, ItemKind::Skill) else {
        return Ok(None);
    };
    let Some(name) = emitted_name(ctx, state, harness) else {
        return Ok(None);
    };
    let body = String::from_utf8_lossy(bytes);
    let marker = match ctx.decl.enabled {
        true => "SKILL.md",
        false => "SKILL.md.disabled",
    };
    let tree = dir.join(&name);
    let artifact = Artifact::Tree {
        canonical: tree.clone(),
        files: vec![(
            PathBuf::from(marker),
            skill_text(&name, &body, ctx.name).into_bytes(),
        )],
        link: None,
    };
    let mut item = declared(ctx, ItemKind::Command, harness, artifact)?;
    item.emitted = Some(EmittedArtifact {
        kind: ItemKind::Skill,
        name,
        paths: vec![tree],
    });
    Ok(Some(item))
}

/// The name the generated skill takes. A real skill keeps its own name, so
/// a command that clashes is renamed and the user is told which name to
/// type; when both renames are taken too, nothing is written rather than
/// something being overwritten.
fn emitted_name(ctx: &ItemCtx, state: &mut DesiredState, harness: HarnessId) -> Option<String> {
    let claimed = claimed_skill_names(ctx, harness);
    if !claimed.contains(ctx.name) {
        return Some(ctx.name.to_owned());
    }
    for suffix in ["__command", "__cmd"] {
        let candidate = format!("{}{suffix}", ctx.name);
        if claimed.contains(&candidate) {
            continue;
        }
        state.warnings.push(ItemWarning {
            kind: ItemKind::Command,
            name: ctx.name.to_owned(),
            harness: Some(harness),
            message: format!(
                "a skill already answers to {}, so the command installs as {candidate}",
                ctx.name
            ),
            remediation: Some(format!(
                "run it as {candidate} on {}, or rename one of the two",
                harness.display_name()
            )),
        });
        return Some(candidate);
    }
    state.refused.push(super::desired::Refused {
        kind: ItemKind::Command,
        name: ctx.name.to_owned(),
        harness,
        reason: format!(
            "skills already hold {name}, {name}__command and {name}__cmd — rename one of them",
            name = ctx.name
        ),
    });
    None
}

/// Skill names this harness must not have taken from it: the ones declared
/// for it here, plus everything the source offers, since declaring one of
/// those is a single edit away.
fn claimed_skill_names(ctx: &ItemCtx, harness: HarnessId) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = ctx
        .manifest
        .skills
        .iter()
        .filter(|(_, decl)| {
            target_harnesses(decl, ctx.manifest, ItemKind::Skill, ctx.scope).contains(&harness)
        })
        .map(|(name, _)| name.clone())
        .collect();
    names.extend(crate::source::list_items(
        ctx.sealed,
        ctx.config,
        ItemKind::Skill,
    ));
    names
}

/// The generated SKILL.md: the frontmatter the loader needs, the banner
/// every generated file carries, then the command body as written.
fn skill_text(emitted: &str, body: &str, name: &str) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{GENERATED_BANNER}\n\n{body}",
        yaml_scalar(emitted),
        yaml_scalar(&description(body, name)),
    )
}

/// One line saying what the command does: its own frontmatter description
/// when it declares one, else its first line of prose.
fn description(body: &str, name: &str) -> String {
    let mut lines = body.lines().peekable();
    let mut in_frontmatter = lines.peek().is_some_and(|line| line.trim() == "---");
    if in_frontmatter {
        lines.next();
    }
    for line in lines {
        let line = line.trim();
        if in_frontmatter {
            if line == "---" {
                in_frontmatter = false;
                continue;
            }
            let Some(value) = line.strip_prefix("description:") else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                return value.to_owned();
            }
            continue;
        }
        let prose = line.trim_start_matches('#').trim();
        if !prose.is_empty() {
            return prose.to_owned();
        }
    }
    format!("command {name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_comes_from_frontmatter_then_prose_then_the_name() {
        assert_eq!(
            description(
                "---\ndescription: Ship it\nmodel: opus\n---\n\n# Ship\n",
                "ship"
            ),
            "Ship it"
        );
        assert_eq!(
            description("\n# Ship the branch\n\nSteps.\n", "ship"),
            "Ship the branch"
        );
        assert_eq!(
            description("---\nmodel: opus\n---\n", "ship"),
            "command ship"
        );
        assert_eq!(description("", "ship"), "command ship");
    }

    #[test]
    fn the_generated_skill_keeps_the_body_and_quotes_a_risky_description() {
        let text = skill_text("ship", "---\ndescription: do: it\n---\n\nBody.\n", "ship");
        assert!(
            text.starts_with("---\nname: ship\ndescription: \"do: it\"\n---\n"),
            "{text}"
        );
        assert!(text.contains(GENERATED_BANNER));
        assert!(text.ends_with("Body.\n"));
    }
}
