// Alias for one release cycle: consuming repos' git-hook entrypoints
// hard-code `vstack guard run <hook>` and fail closed without it.
use std::process::ExitCode;

fn main() -> ExitCode {
    kendex_cli::main()
}
