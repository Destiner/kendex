use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use vstack_core::env::Env;
use vstack_core::guard::{self, GuardCtx, Outcome};

use super::out;

/// Exit taxonomy, the family contract: 0 clean, 1 violations, 2 the check
/// could not run. Both nonzero verdicts block a commit.
#[derive(Subcommand)]
pub enum GuardCommand {
    /// Run a hook lane — what the installed entrypoints call
    Run {
        /// pre-commit | commit-msg
        hook: String,
        /// commit-msg only: the message file git passed
        message_file: Option<PathBuf>,
    },
    /// Tighten-only file-size gate over tracked files
    #[command(name = "size-ratchet")]
    SizeRatchet {
        /// Write the first baseline; refuses if one exists
        #[arg(long)]
        seed: bool,
        /// Tighten the baseline to current reality (never adds, never raises)
        #[arg(long, conflicts_with = "seed")]
        update: bool,
    },
    /// Flat ban on TODO/FIXME/HACK/XXX work markers in tracked files
    #[command(name = "todo-ban")]
    TodoBan,
    /// Newly added files over the byte ceiling fail (lockfiles exempt)
    #[command(name = "byte-ceiling")]
    ByteCeiling,
    /// Blanket lint suppressions fail flat; bare rust allows ratchet
    #[command(name = "suppression-ban")]
    SuppressionBan {
        /// Tighten the baseline to current reality (never adds, never raises)
        #[arg(long)]
        update: bool,
    },
    /// Conventional-commit gate over one message file (or stdin)
    #[command(name = "commit-msg")]
    CommitMsg { file: Option<PathBuf> },
    /// Install the owned hooks directory and point core.hooksPath at it
    Install,
    /// Release this worktree's lease; disarm when the last one goes
    Uninstall,
    /// Convert v1 guard settings into [guards] tables — one explicit pass
    #[command(name = "import-v1")]
    ImportV1,
}

/// The message as commit-msg judges it. The file argument is resolved
/// against the invoker's working directory before anything else — git
/// passes `.git/COMMIT_EDITMSG` relative to where it ran the hook.
fn read_message(file: Option<&PathBuf>) -> Result<String, String> {
    match file {
        None => {
            use std::io::Read;
            let mut message = String::new();
            std::io::stdin()
                .read_to_string(&mut message)
                .map_err(|e| format!("could not read the commit message from stdin: {e}"))?;
            Ok(message)
        }
        Some(path) => {
            let absolute = match path.is_absolute() {
                true => path.clone(),
                false => std::env::current_dir()
                    .map_err(|e| e.to_string())?
                    .join(path),
            };
            std::fs::read_to_string(&absolute).map_err(|e| {
                format!(
                    "could not read the commit message file {}: {e}",
                    absolute.display()
                )
            })
        }
    }
}

fn verdict(outcome: Result<Outcome, vstack_core::error::CoreError>) -> ExitCode {
    match outcome {
        Ok(outcome) => {
            for line in &outcome.lines {
                out(line);
            }
            match outcome.violations {
                0 => ExitCode::SUCCESS,
                _ => ExitCode::from(1),
            }
        }
        Err(error) => {
            out(&format!("error: {error}"));
            ExitCode::from(2)
        }
    }
}

pub fn run(env: &Env, command: GuardCommand) -> Result<ExitCode, Box<dyn std::error::Error>> {
    // Hook installs mutate repository state through the ordinary journaled
    // machinery; everything else is a read-only verdict over the index.
    match &command {
        GuardCommand::Install => {
            let report = vstack_core::githooks::install(env, &std::env::current_dir()?)?;
            for line in report.lines {
                out(&line);
            }
            return Ok(ExitCode::SUCCESS);
        }
        GuardCommand::Uninstall => {
            let report = vstack_core::githooks::uninstall(env, &std::env::current_dir()?)?;
            for line in report.lines {
                out(&line);
            }
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    // The message is read before the repository is even bound: the path is
    // relative to the invoker's directory, never the repo root.
    let message = match &command {
        GuardCommand::Run { hook, message_file } if hook == "commit-msg" => {
            Some(read_message(message_file.as_ref()))
        }
        GuardCommand::CommitMsg { file } => Some(read_message(file.as_ref())),
        _ => None,
    };
    let message = match message {
        Some(Ok(message)) => Some(message),
        Some(Err(error)) => {
            out(&format!("error: commit-msg: {error}"));
            return Ok(ExitCode::from(2));
        }
        None => None,
    };

    let ctx = match GuardCtx::bind(&std::env::current_dir()?) {
        Ok(ctx) => ctx,
        Err(error) => {
            out(&format!("error: {error}"));
            return Ok(ExitCode::from(2));
        }
    };
    let policy = || guard::settings::Policy::load(&ctx, "guard");

    Ok(match command {
        GuardCommand::Run { hook, .. } => match hook.as_str() {
            "pre-commit" => {
                let report = guard::run_pre_commit(&ctx);
                for line in &report.lines {
                    out(line);
                }
                ExitCode::from(report.exit_code())
            }
            "commit-msg" => verdict(policy().and_then(|policy| {
                guard::commit_msg::run(&ctx, &policy, message.as_deref().unwrap_or_default())
            })),
            other => {
                out(&format!(
                    "error: unknown hook '{other}' (pre-commit | commit-msg)"
                ));
                ExitCode::from(2)
            }
        },
        GuardCommand::SizeRatchet { seed, update } => {
            let mode = match (seed, update) {
                (true, _) => guard::size_ratchet::Mode::Seed,
                (_, true) => guard::size_ratchet::Mode::Update,
                _ => guard::size_ratchet::Mode::Check,
            };
            verdict(policy().and_then(|policy| guard::size_ratchet::run(&ctx, &policy, mode)))
        }
        GuardCommand::TodoBan => {
            verdict(policy().and_then(|policy| guard::todo_ban(&ctx, &policy)))
        }
        GuardCommand::ByteCeiling => {
            verdict(policy().and_then(|policy| guard::byte_ceiling::run(&ctx, &policy)))
        }
        GuardCommand::SuppressionBan { update } => {
            verdict(policy().and_then(|policy| guard::suppression_ban::run(&ctx, &policy, update)))
        }
        GuardCommand::CommitMsg { .. } => verdict(policy().and_then(|policy| {
            guard::commit_msg::run(&ctx, &policy, message.as_deref().unwrap_or_default())
        })),
        GuardCommand::ImportV1 => {
            let report = guard::import::run(&ctx)?;
            for line in &report.lines {
                out(line);
            }
            ExitCode::SUCCESS
        }
        GuardCommand::Install | GuardCommand::Uninstall => unreachable!("handled above"),
    })
}
