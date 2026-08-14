import { beforeEach, describe, expect, it } from "vitest";
import { useNavStore } from "./nav";

describe("nav store", () => {
  beforeEach(() => {
    useNavStore.setState({
      page: "home",
      scope: "all",
      libraryTab: "installed",
      toolsTab: "tools",
      libraryFilter: null,
    });
  });

  it("hands off a tool + kind filter to Library and clears it on request", () => {
    useNavStore.getState().goToLibrary({ tool: "claude", kind: "hook" });

    const state = useNavStore.getState();
    expect(state.page).toBe("library");
    expect(state.libraryTab).toBe("installed");
    expect(state.libraryFilter).toEqual({ tool: "claude", kind: "hook" });

    state.clearLibraryFilter();
    expect(useNavStore.getState().libraryFilter).toBeNull();
  });

  it("plays the plain tab-switch case with no filter", () => {
    useNavStore.getState().goToLibrary({ tab: "add" });

    const state = useNavStore.getState();
    expect(state.page).toBe("library");
    expect(state.libraryTab).toBe("add");
    expect(state.libraryFilter).toBeNull();
  });

  it("defaults to the installed tab when only a filter is given", () => {
    useNavStore.getState().goToLibrary({ kind: "skill" });

    expect(useNavStore.getState().libraryTab).toBe("installed");
  });
});
