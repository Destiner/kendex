import { describe, expect, it } from "vitest";
import { problemsFooterLabel } from "./error-copy";

describe("problemsFooterLabel", () => {
  it("singularizes exactly one problem", () => {
    expect(problemsFooterLabel(1)).toBe("1 problem");
  });

  it("pluralizes everything else, including zero", () => {
    expect(problemsFooterLabel(0)).toBe("0 problems");
    expect(problemsFooterLabel(3)).toBe("3 problems");
  });
});
