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
      history: [],
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

  it("pushes the prior page onto history on a cross-page nav", () => {
    useNavStore.getState().goToLibrary();

    expect(useNavStore.getState().history).toEqual([
      { page: "home", libraryTab: "installed", toolsTab: "tools" },
    ]);
  });

  it("does not push when goToLibrary only switches tabs on the same page", () => {
    useNavStore.getState().goToLibrary();
    useNavStore.getState().goToLibrary({ tab: "add" });

    expect(useNavStore.getState().history).toHaveLength(1);
  });

  it("back() pops history and restores the prior page and tab", () => {
    useNavStore.setState({ toolsTab: "projects" });
    useNavStore.getState().goToLibrary({ tab: "add" });
    useNavStore.getState().back();

    const state = useNavStore.getState();
    expect(state.page).toBe("home");
    expect(state.toolsTab).toBe("projects");
    expect(state.history).toEqual([]);
  });

  it("back() clears any pending library filter", () => {
    useNavStore.getState().goToTools("tools");
    useNavStore.getState().goToLibrary({ tool: "claude", kind: "hook" });
    useNavStore.getState().back();

    expect(useNavStore.getState().libraryFilter).toBeNull();
  });

  it("is a no-op when history is empty", () => {
    useNavStore.getState().back();

    expect(useNavStore.getState().page).toBe("home");
  });

  it("setPage resets the history stack and clears any pending filter", () => {
    useNavStore.getState().goToLibrary({ tool: "claude", kind: "hook" });
    useNavStore.getState().setPage("settings");

    const state = useNavStore.getState();
    expect(state.history).toEqual([]);
    expect(state.libraryFilter).toBeNull();
  });

  it("caps the history stack so it never grows without bound", () => {
    for (let i = 0; i < 25; i++) {
      useNavStore.getState().goToLibrary();
      useNavStore.getState().goToTools("tools");
    }

    expect(useNavStore.getState().history.length).toBeLessThanOrEqual(20);
  });
});
