//! User-typed paths enter the app layer here: register_project,
//! discover_projects, and harness-root overrides all take a path a person
//! typed by hand rather than one a picker or the scanner produced, and a
//! shell would have expanded a leading `~` before any of those ever saw it
//! — the GUI has no shell in front of it, so the app layer is where that
//! expansion has to happen instead.

use std::path::{Path, PathBuf};

/// Expands a leading `~/` or a lone `~` against `home`. `~user` (naming
/// another account's home) is left untouched — silently redirecting it to
/// the current user's home would point the app at the wrong directory
/// without saying so.
pub fn expand_tilde(home: &Path, input: &str) -> PathBuf {
    if input == "~" {
        return home.to_path_buf();
    }
    match input.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_tilde_slash_resolves_against_home() {
        let home = Path::new("/home/pat");
        assert_eq!(
            expand_tilde(home, "~/dev/hyprtrade"),
            PathBuf::from("/home/pat/dev/hyprtrade")
        );
    }

    #[test]
    fn a_lone_tilde_is_home_itself() {
        let home = Path::new("/home/pat");
        assert_eq!(expand_tilde(home, "~"), PathBuf::from("/home/pat"));
    }

    #[test]
    fn another_users_tilde_passes_through_unexpanded() {
        let home = Path::new("/home/pat");
        assert_eq!(expand_tilde(home, "~alex/dev"), PathBuf::from("~alex/dev"));
    }

    #[test]
    fn absolute_and_relative_paths_are_untouched() {
        let home = Path::new("/home/pat");
        assert_eq!(
            expand_tilde(home, "/opt/dev/hyprtrade"),
            PathBuf::from("/opt/dev/hyprtrade")
        );
        assert_eq!(
            expand_tilde(home, "dev/hyprtrade"),
            PathBuf::from("dev/hyprtrade")
        );
    }
}
