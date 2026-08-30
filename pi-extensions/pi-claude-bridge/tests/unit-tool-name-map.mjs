import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { isPiDispatchable, mapToolName } from "../src/index.ts";

describe("tool name mapping", () => {
	it("maps known Claude builtin names to Pi tool names", () => {
		assert.equal(mapToolName("Read"), "read");
	});

	it("maps MCP-qualified custom tool names back to Pi tool names", () => {
		const map = new Map([["mcp__custom-tools__grep", "grep"]]);
		assert.equal(mapToolName("mcp__custom-tools__grep", map), "grep");
		assert.equal(mapToolName("mcp__custom-tools__Grep", map), "grep");
		assert.equal(mapToolName("mcp__custom_tools__grep"), "grep");
		assert.equal(mapToolName("mcp/custom-tools/grep"), "grep");
		assert.equal(mapToolName("mcp/custom_tools/grep"), "grep");
	});

	it("rejects tool names that cannot belong to the bridged manifest", () => {
		const map = new Map([["mcp__custom-tools__read", "read"]]);
		assert.equal(isPiDispatchable("bash", map), false);
		assert.equal(isPiDispatchable("mcp__filesystem__read_file", map), false);
		assert.equal(isPiDispatchable("mcp__custom-tools__read", map), true);
		assert.equal(isPiDispatchable("bash", new Map()), true);
	});
});
