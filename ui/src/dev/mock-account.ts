// Sign-in state for the mock bridge: the device flow "approves" on the
// second poll, and submissions exist only while signed in.
import type { Handler } from "./mock-state";

let signedIn = false;
let polls = 0;

export function isSignedIn(): boolean {
  return signedIn;
}

export const accountHandlers: Record<string, Handler> = {
  account_status: () => ({ signedIn, endpoint: "https://kendex.ai" }),
  account_login_start: () => {
    polls = 0;
    return {
      deviceCode: "kxd_mock",
      userCode: "ABCD-2345",
      verificationUrl: "https://kendex.ai/device?code=ABCD-2345",
      intervalSeconds: 1,
    };
  },
  account_login_poll: () => {
    polls += 1;
    if (polls < 2) return "pending";
    signedIn = true;
    return "signed";
  },
  account_logout: () => {
    signedIn = false;
    return null;
  },
  open_url: () => null,
};
