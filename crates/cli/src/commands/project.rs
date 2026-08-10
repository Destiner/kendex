use std::path::PathBuf;

use clap::Subcommand;
use vstack_core::env::Env;
use vstack_core::error::CoreError;
use vstack_core::{discover, settings};

use super::{CliResult, out};

#[derive(Subcommand)]
pub enum ProjectCommand {
    /// Register a project directory
    Add { path: PathBuf },
    /// Drop a project from the registry (its files are untouched)
    Remove { path: PathBuf },
    /// List registered projects
    List,
    /// Walk a directory for harness-marked projects
    Discover {
        root: PathBuf,
        /// Register every project found
        #[arg(long)]
        register: bool,
    },
}

pub fn run(env: &Env, cmd: ProjectCommand) -> CliResult {
    match cmd {
        ProjectCommand::Add { path } => {
            settings::register_project(env, &path)?;
            out(&format!("registered {}", path.display()));
        }
        ProjectCommand::Remove { path } => {
            settings::unregister_project(env, &path)?;
            out(&format!("removed {}", path.display()));
        }
        ProjectCommand::List => {
            for project in settings::load(env)?.projects {
                let missing = if project.is_dir() { "" } else { "  (missing)" };
                out(&format!("{}{missing}", project.display()));
            }
        }
        ProjectCommand::Discover { root, register } => {
            for found in discover::discover_projects(&root)? {
                if register {
                    match settings::register_project(env, &found) {
                        Ok(_) => out(&format!("registered {}", found.display())),
                        Err(CoreError::ProjectAlreadyRegistered { .. }) => {
                            out(&format!("already registered {}", found.display()));
                        }
                        Err(e) => return Err(e.into()),
                    }
                } else {
                    out(&format!("{}", found.display()));
                }
            }
        }
    }
    Ok(())
}
