import { MCP_SERVER_NAME, MCP_TOOL_PREFIX } from "./skills.js";

const SDK_TO_PI_TOOL_NAME: Record<string, string> = {
	read: "read", write: "write", edit: "edit", bash: "bash",
};

const BRIDGED_TOOL_PREFIXES = [
	MCP_TOOL_PREFIX,
	`mcp__${MCP_SERVER_NAME.replace(/-/g, "_")}__`,
	`mcp/${MCP_SERVER_NAME}/`,
	`mcp/${MCP_SERVER_NAME.replace(/-/g, "_")}/`,
];

// --- Provider helpers: tool name mapping ---

export function isPiDispatchable(name: string, customToolNameToPi?: Map<string, string>): boolean {
	if (!name) return false;
	const normalized = name.toLowerCase();
	if (customToolNameToPi?.has(name) || customToolNameToPi?.has(normalized)) return true;
	if (BRIDGED_TOOL_PREFIXES.some((prefix) => normalized.startsWith(prefix))) return true;
	// A foreign MCP namespace belongs to a child-loaded server, not Pi's bridge.
	if (normalized.startsWith("mcp__")) return false;
	// Bare SDK aliases are naming slips when the child has a bridged manifest.
	if (customToolNameToPi?.size && SDK_TO_PI_TOOL_NAME[normalized]) return false;
	return true;
}

export function mapToolName(name: string, customToolNameToPi?: Map<string, string>): string {
	const normalized = name.toLowerCase();
	const builtin = SDK_TO_PI_TOOL_NAME[normalized];
	if (builtin) return builtin;
	if (customToolNameToPi) {
		const mapped = customToolNameToPi.get(name) ?? customToolNameToPi.get(normalized);
		if (mapped) return mapped;
	}
	for (const prefix of BRIDGED_TOOL_PREFIXES) {
		if (normalized.startsWith(prefix)) return normalized.slice(prefix.length);
	}
	return name;
}

// Renames for Claude Code SDK param names that differ from pi's native names.
// Keys not listed here pass through unchanged, so new pi params work automatically.
const SDK_KEY_RENAMES: Record<string, Record<string, string>> = {
	read:  { file_path: "path" },
	write: { file_path: "path" },
	edit:  { file_path: "path", old_string: "oldText", new_string: "newText", old_text: "oldText", new_text: "newText" },
};

// Maps SDK tool args to pi tool args via key renaming + pass-through.
// Pi's own prepareArguments hooks handle any structural transforms (e.g. edit oldText/newText → edits[]).
export function mapToolArgs(
	toolName: string, args: Record<string, unknown> | undefined,
): Record<string, unknown> {
	const input = args ?? {};
	const renames = SDK_KEY_RENAMES[toolName.toLowerCase()];
	const result: Record<string, unknown> = {};
	for (const [key, value] of Object.entries(input)) {
		const piKey = renames?.[key] ?? key;
		if (!(piKey in result)) result[piKey] = value; // first alias wins
	}
	// Pi bash has no default timeout; add a safety default
	if (toolName.toLowerCase() === "bash" && result.timeout == null) {
		result.timeout = 120;
	}
	return result;
}
