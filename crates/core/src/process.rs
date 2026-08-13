//! Every external process this crate launches is built here (invariant 13):
//! environment that can redirect it is cleared, every prompt path is closed,
//! and every call carries a timeout. A per-call-site discipline misses call
//! sites — v1 had 27 unguarded `Command::new("git")` invocations — so the
//! raw pattern is guard-banned everywhere but this file.
//!
//! The threat that shapes `git_in`: a catalog repository is other people's
//! data. Its `.git/config` may set `core.worktree` to a directory outside
//! the cache, and a refresh (`reset --hard`) then writes the repository's
//! files over whatever lives there — the user's own work, one directory up.
//! Clearing `GIT_WORK_TREE` does not help, because the setting comes from
//! the downloaded repository rather than the environment. Only the command
//! line outranks config, so operations inside a cache pin `--git-dir` and
//! `--work-tree` explicitly and the hostile setting is ignored.

use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::error::{CoreError, Result};

/// Long enough for a cold clone over a slow link, short enough that a wedged
/// call surfaces as an error instead of a frozen window.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

const POLL: Duration = Duration::from_millis(10);

/// Environment that points git at a different repository than the caller
/// named — inherited from whatever launched the app, including another
/// harness mid-operation.
const GIT_REDIRECTS: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
];

pub struct Hardened {
    command: Command,
    /// What the caller asked for, for error messages. Plumbing arguments
    /// this module adds are deliberately left out.
    label: String,
    timeout: Duration,
}

impl Hardened {
    pub fn git(args: &[&str], cwd: Option<&Path>) -> Hardened {
        let mut hardened = Hardened::git_command(owned(args), cwd);
        hardened.label = format!("git {}", args.join(" "));
        hardened
    }

    /// git against a downloaded repository. The working tree is pinned on
    /// the command line, where it outranks a `core.worktree` the repository
    /// ships, so the call cannot reach outside `repo`.
    pub fn git_in(repo: &Path, args: &[&str]) -> Hardened {
        let mut pinned = vec![
            OsString::from("--git-dir"),
            repo.join(".git").into_os_string(),
            OsString::from("--work-tree"),
            repo.as_os_str().to_owned(),
        ];
        pinned.extend(owned(args));
        let mut hardened = Hardened::git_command(pinned, Some(repo));
        hardened.label = format!("git {}", args.join(" "));
        hardened
    }

    pub fn npm(args: &[&str], cwd: Option<&Path>) -> Hardened {
        let mut hardened = Hardened::new("npm", owned(args));
        if let Some(cwd) = cwd {
            hardened.command.current_dir(cwd);
        }
        hardened
    }

    pub fn gh(args: &[&str]) -> Hardened {
        Hardened::new("gh", owned(args))
    }

    pub fn curl(args: &[&str]) -> Hardened {
        Hardened::new("curl", owned(args))
    }

    pub fn timeout(mut self, timeout: Duration) -> Hardened {
        self.timeout = timeout;
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn run(mut self) -> Result<Output> {
        let mut child = match self.command.spawn() {
            Ok(child) => child,
            Err(error) => return Err(CoreError::io(&self.label, error)),
        };
        let (Some(mut stdout), Some(mut stderr)) = (child.stdout.take(), child.stderr.take())
        else {
            return Err(CoreError::io(
                &self.label,
                io::Error::other("child was spawned without pipes"),
            ));
        };
        // Drained on threads: a child that fills a pipe buffer would block
        // forever while we sat polling for its exit.
        let reading_out = std::thread::spawn(move || read(&mut stdout));
        let reading_err = std::thread::spawn(move || read(&mut stderr));

        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(error) => return Err(CoreError::io(&self.label, error)),
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CoreError::io(
                    &self.label,
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("no result after {:?}", self.timeout),
                    ),
                ));
            }
            std::thread::sleep(POLL);
        };
        Ok(Output {
            status,
            stdout: collect(reading_out, &self.label)?,
            stderr: collect(reading_err, &self.label)?,
        })
    }

    fn git_command(args: Vec<OsString>, cwd: Option<&Path>) -> Hardened {
        let mut hardened = Hardened::new("git", args);
        for variable in GIT_REDIRECTS {
            hardened.command.env_remove(variable);
        }
        hardened.command.env("GIT_TERMINAL_PROMPT", "0");
        hardened
            .command
            .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes");
        if let Some(cwd) = cwd {
            hardened.command.current_dir(cwd);
        }
        hardened
    }

    fn new(program: &str, args: Vec<OsString>) -> Hardened {
        let label = std::iter::once(program.to_owned())
            .chain(args.iter().map(|arg| arg.to_string_lossy().into_owned()))
            .collect::<Vec<_>>()
            .join(" ");
        let mut command = Command::new(program);
        command
            .args(&args)
            // No prompt can block: nothing to read from.
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Hardened {
            command,
            label,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    #[cfg(test)]
    fn program(program: &str, args: &[&str]) -> Hardened {
        Hardened::new(program, owned(args))
    }
}

fn owned(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn read(pipe: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    pipe.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn collect(reader: JoinHandle<io::Result<Vec<u8>>>, label: &str) -> Result<Vec<u8>> {
    match reader.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(error)) => Err(CoreError::io(label, error)),
        Err(_) => Err(CoreError::io(
            label,
            io::Error::other("output reader panicked"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::OsStr;
    use std::fs;

    fn child_env(hardened: &Hardened) -> HashMap<&OsStr, Option<&OsStr>> {
        hardened.command.get_envs().collect()
    }

    #[test]
    fn git_runs_without_redirecting_environment_and_without_prompts() {
        let hardened = Hardened::git(&["status"], None);
        let env = child_env(&hardened);
        for variable in GIT_REDIRECTS {
            assert_eq!(
                env.get(OsStr::new(variable)),
                Some(&None),
                "{variable} must be removed from the child"
            );
        }
        assert_eq!(
            env[OsStr::new("GIT_TERMINAL_PROMPT")],
            Some(OsStr::new("0"))
        );
        assert_eq!(
            env[OsStr::new("GIT_SSH_COMMAND")],
            Some(OsStr::new("ssh -oBatchMode=yes"))
        );
    }

    #[test]
    fn errors_name_the_call_the_caller_asked_for_not_the_pinning() {
        let repo = Path::new("/nowhere/cache");
        assert_eq!(
            Hardened::git_in(repo, &["fetch", "origin"]).label(),
            "git fetch origin"
        );
    }

    /// A cached repository whose own config points its working tree at a
    /// sibling directory. Un-pinned, `git reset --hard` here overwrites the
    /// sibling's files with the repository's; pinned, it cannot see them.
    #[test]
    fn a_hostile_core_worktree_cannot_reach_outside_the_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache");
        let victim = tmp.path().join("victim");
        fs::create_dir_all(cache.join("skills/gh")).unwrap();
        fs::create_dir_all(victim.join("skills/gh")).unwrap();
        fs::write(cache.join("skills/gh/SKILL.md"), "from the catalog\n").unwrap();
        for args in [
            vec!["init", "--quiet", "-b", "main"],
            vec!["add", "."],
            vec![
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "--quiet",
                "-m",
                "one",
            ],
            vec!["config", "core.worktree", &victim.display().to_string()],
        ] {
            assert!(
                Hardened::git(&args, Some(&cache))
                    .run()
                    .unwrap()
                    .status
                    .success()
            );
        }
        let precious = victim.join("skills/gh/SKILL.md");
        fs::write(&precious, "the user's own work\n").unwrap();
        fs::write(cache.join("skills/gh/SKILL.md"), "locally edited\n").unwrap();

        let reset = Hardened::git_in(&cache, &["reset", "--hard", "HEAD", "--quiet"])
            .run()
            .unwrap();

        assert!(reset.status.success());
        assert_eq!(
            fs::read_to_string(&precious).unwrap(),
            "the user's own work\n"
        );
        assert_eq!(
            fs::read_to_string(cache.join("skills/gh/SKILL.md")).unwrap(),
            "from the catalog\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_call_that_outlives_its_timeout_is_killed() {
        let started = Instant::now();
        let error = Hardened::program("/bin/sleep", &["5"])
            .timeout(Duration::from_millis(200))
            .run()
            .unwrap_err();
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "waited too long"
        );
        let CoreError::Io { source, .. } = error else {
            panic!("timeout must report as an io error");
        };
        assert_eq!(source.kind(), io::ErrorKind::TimedOut);
    }

    #[cfg(unix)]
    #[test]
    fn a_child_reading_stdin_gets_nothing_instead_of_waiting() {
        let output = Hardened::program("/bin/cat", &[])
            .timeout(Duration::from_secs(5))
            .run()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
    }
}
