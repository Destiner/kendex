// A plain-English one-liner per safety rule the engine actually emits
// (crates/core/src/quality/rules — the `id()` of each rule); unknown rules
// fall back to the engine's own message rather than a raw rule id.
const FINDING_HEADLINES: Record<string, string> = {
  "plaintext-secrets": "Contains a real credential in plain text",
  "credential-theft": "Reads a credential and sends it somewhere",
  "dangerous-commands": "Contains a command that could do real damage",
  "prompt-injection": "Tries to override the assistant's instructions",
  rce: "Downloads and runs code from the internet",
  "safety-bypass": "Tries to turn off safety checks",
  "supply-chain": "Pulled from a source anyone could publish to",
  "broad-permissions": "Asks for more access than it needs",
  "mcp-command-injection": "Lets outside text run commands",
  "obfuscated-content": "Contains text disguised to look like something else",
  "undecodable-content": "Contains bytes that can't be read as text",
  "plugin-lifecycle-scripts": "Runs its own scripts when installed or started",
  "plugin-source-trust":
    "Installed from an untracked source, so updates can't be checked",
};

export function findingHeadline(rule: string, fallbackMessage: string): string {
  return FINDING_HEADLINES[rule] ?? fallbackMessage;
}
