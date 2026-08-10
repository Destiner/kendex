mod commands;
mod scope;

use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vstack_core::env::Env;

use commands::project::ProjectCommand;
use scope::ScopeFilter;

#[derive(Parser)]
#[command(
    name = "vstack",
    version,
    about = "Skills, agents, hooks. Cross-harness."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Register, list, and discover vstack-enabled projects
    #[command(subcommand)]
    Project(ProjectCommand),

    /// List everything observed on this machine
    #[command(alias = "ls")]
    List {
        /// Shortcut for --scope global
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
        /// Filter by harness id
        #[arg(long)]
        harness: Option<String>,
    },

    /// Detection sanity: harnesses found, items observed, unreadable surfaces
    Check {
        /// Shortcut for --scope global
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> commands::CliResult {
    let env = Env::detect()?;
    match cli.command {
        Command::Project(cmd) => commands::project::run(&env, cmd),
        Command::List {
            global,
            scope,
            harness,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::list::run(&env, filter, harness)
        }
        Command::Check { global, scope } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::check::run(&env, filter)
        }
    }
}
