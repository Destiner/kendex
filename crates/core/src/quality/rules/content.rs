//! Instructions aimed at the model rather than at the user, and commands
//! that fetch and run code or carry credentials off the machine.

use super::{AUTHORED, AuditRule, Finding, Line, Outcome, Prepared, Severity, at, scan_docs};

pub(super) fn rules() -> Vec<Box<dyn AuditRule>> {
    vec![
        Box::new(PromptInjection),
        Box::new(Rce),
        Box::new(CredentialTheft),
    ]
}

/// Phrases whose only purpose is to talk past the instructions a harness
/// already gave the model.
///
/// HarnessKit's seventh pattern matched raw zero-width characters and could
/// never fire, because its own deobfuscation removed them first. Here that
/// signal is `obfuscated-content`, which reports on the deobfuscation
/// itself and therefore still sees it.
const INJECTION: &[&str] = &[
    "ignore previous instructions",
    "ignore all previous instructions",
    "ignore the previous instructions",
    "ignore all prior instructions",
    "ignore the above instructions",
    "disregard prior",
    "disregard previous",
    "disregard the above",
    "you are now a",
    "you are now an",
    "new system prompt",
    "override system prompt",
    "override the system prompt",
    "override safety prompt",
    "override the safety prompt",
    "[system]",
];

struct PromptInjection;

impl AuditRule for PromptInjection {
    fn id(&self) -> &'static str {
        "prompt-injection"
    }

    fn check(&self, prepared: &Prepared) -> Outcome {
        scan_docs(prepared, AUTHORED, |doc, line, findings| {
            for phrase in INJECTION {
                if !line.has(phrase) {
                    continue;
                }
                findings.push(Finding {
                    rule: self.id().to_owned(),
                    severity: line.weigh(phrase, Severity::Critical),
                    location: at(doc, line),
                    message: format!(
                        "this line tells the model to set aside the instructions it was given (\"{phrase}\")"
                    ),
                    remediation:
                        "delete the line; if it is quoting an attack for documentation, say so in prose instead of writing the instruction out"
                            .to_owned(),
                });
            }
        })
    }
}

struct Rce;

impl AuditRule for Rce {
    fn check(&self, prepared: &Prepared) -> Outcome {
        scan_docs(prepared, AUTHORED, |doc, line, findings| {
            let Some((needle, what)) = fetch_and_run(line) else {
                return;
            };
            findings.push(Finding {
                rule: self.id().to_owned(),
                severity: line.weigh(needle, Severity::Critical),
                location: at(doc, line),
                message: format!("this line {what}, so whatever the far end serves is what runs"),
                remediation:
                    "download to a file, show the user what it contains, and run it as a separate step they can refuse"
                        .to_owned(),
            });
        })
    }

    fn id(&self) -> &'static str {
        "rce"
    }
}

/// The needle that matched and a plain description of it.
fn fetch_and_run(line: &Line) -> Option<(&'static str, &'static str)> {
    const SHELLS: &[&str] = &["| sh", "|sh", "| bash", "|bash", "| zsh", "| python"];
    let downloads = ["curl", "wget"].iter().find(|verb| line.has(verb));
    if let Some(download) = downloads {
        if let Some(pipe) = SHELLS.iter().find(|shell| line.has(shell)) {
            return Some((pipe, "pipes a download straight into a shell"));
        }
        if line.has("/tmp/")
            && ["&& sh", "&& bash", "chmod +x"]
                .iter()
                .any(|run| line.has(run))
        {
            return Some((download, "downloads a file and then executes it"));
        }
    }
    if line.has("base64") && line.has("|") && (line.has("-d") || line.has("--decode")) {
        return Some(("base64", "decodes hidden text and pipes it onward"));
    }
    line.has("eval(")
        .then_some(("eval(", "hands a built-up string to an interpreter"))
}

/// Files that hold credentials, and the verbs that would send them
/// somewhere.
///
/// Two calibrations against HarnessKit. It counted a bare `http` as an
/// outbound verb, which makes every page documenting an AWS path and
/// linking to AWS docs a Critical finding, so the verbs here are the ones
/// that actually send. And it matched the bare word `credentials`, which
/// fires on the sentence "bad credentials" in a troubleshooting section —
/// the paths below are all path-shaped, and `.aws/` already covers the file
/// that word was aiming at.
const CREDENTIAL_FILES: &[&str] = &[".ssh/", ".aws/", ".netrc", ".pgpass", ".env"];

const OUTBOUND: &[&str] = &[
    "curl",
    "wget",
    "nc ",
    "netcat",
    "-x post",
    ".post(",
    "requests.post",
    "fetch(",
    "urlopen",
];

struct CredentialTheft;

impl AuditRule for CredentialTheft {
    fn id(&self) -> &'static str {
        "credential-theft"
    }

    fn check(&self, prepared: &Prepared) -> Outcome {
        scan_docs(prepared, AUTHORED, |doc, line, findings| {
            let Some(file) = CREDENTIAL_FILES.iter().find(|path| line.has(path)) else {
                return;
            };
            let sends = OUTBOUND.iter().find(|verb| line.has(verb));
            let (base, message) = match sends {
                Some(verb) => (
                    Severity::Critical,
                    format!(
                        "this line reads `{file}` and sends it away with `{}`",
                        verb.trim()
                    ),
                ),
                // Naming a credential path is what documentation does;
                // moving what is in it is what theft does.
                None => (
                    Severity::Medium,
                    format!("this line reads `{file}`, which holds credentials"),
                ),
            };
            findings.push(Finding {
                rule: self.id().to_owned(),
                severity: line.weigh(file, base),
                location: at(doc, line),
                message,
                remediation:
                    "read credentials from the environment the user already set up, and never move them off the machine"
                        .to_owned(),
            });
        })
    }
}
