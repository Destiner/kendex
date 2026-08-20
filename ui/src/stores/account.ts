// Sign-in state and the device flow: the store owns the poll timer so a
// closed dialog stops asking, and submissions the Mine rows join on.
import { create } from "zustand";
import { commands, type SubmissionRow } from "@/bindings";

interface AccountState {
  signedIn: boolean | null;
  /** The in-flight device flow, when one is showing. */
  userCode: string | null;
  verificationUrl: string | null;
  signingIn: boolean;
  error: string | null;
  submissions: SubmissionRow[] | null;

  load: () => Promise<void>;
  /** Starts the device flow and polls until signed, denied or closed. */
  signIn: () => Promise<void>;
  cancelSignIn: () => void;
  signOut: () => Promise<void>;
  loadSubmissions: () => Promise<void>;
}

/** Bumped to abandon a poll loop whose dialog was closed. */
let generation = 0;

const wait = (seconds: number) =>
  new Promise((resolve) => setTimeout(resolve, seconds * 1000));

export const useAccountStore = create<AccountState>((set, get) => ({
  signedIn: null,
  userCode: null,
  verificationUrl: null,
  signingIn: false,
  error: null,
  submissions: null,

  load: async () => {
    const status = await commands.accountStatus();
    if (status.status === "ok") set({ signedIn: status.data.signedIn });
  },

  signIn: async () => {
    generation += 1;
    const mine = generation;
    set({ signingIn: true, error: null });
    const started = await commands.accountLoginStart();
    if (started.status === "error") {
      set({ signingIn: false, error: started.error });
      return;
    }
    set({
      userCode: started.data.userCode,
      verificationUrl: started.data.verificationUrl,
    });
    void commands.openUrl(started.data.verificationUrl);
    let interval = Math.max(1, started.data.intervalSeconds);
    while (generation === mine) {
      await wait(interval);
      if (generation !== mine) return;
      const polled = await commands.accountLoginPoll(started.data.deviceCode);
      if (generation !== mine) return;
      if (polled.status === "error") {
        set({ signingIn: false, userCode: null, error: polled.error });
        return;
      }
      if (polled.data === "signed") {
        set({ signingIn: false, userCode: null, signedIn: true });
        return;
      }
      if (polled.data === "slow-down") interval += 5;
    }
  },

  cancelSignIn: () => {
    generation += 1;
    set({ signingIn: false, userCode: null, verificationUrl: null });
  },

  signOut: async () => {
    const out = await commands.accountLogout();
    if (out.status === "error") {
      set({ error: out.error });
      return;
    }
    set({ signedIn: false, submissions: null, error: null });
  },

  loadSubmissions: async () => {
    if (!get().signedIn) return;
    const rows = await commands.mineSubmissions();
    if (rows.status === "ok") set({ submissions: rows.data });
  },
}));
