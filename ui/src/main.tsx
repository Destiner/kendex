import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";

async function start(): Promise<void> {
  // Dev-only: VITE_MOCK=1 swaps the Tauri bridge for an in-memory mock so
  // browser automation can drive the real pages outside the app window.
  if (import.meta.env.DEV && import.meta.env.VITE_MOCK) {
    await import("./dev/mock");
  }
  const root = document.getElementById("root");
  if (!root) {
    throw new Error("missing #root element");
  }
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
void start();
