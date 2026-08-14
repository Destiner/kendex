import { toast } from "sonner";
import { commands } from "@/bindings";

/**
 * Opens the native folder picker and returns the chosen path, or null if
 * the user cancelled — every path-typing input on Settings and Tools &
 * Projects wires its Browse… button through this so cancel silently does
 * nothing and a real failure still surfaces as a toast.
 */
export async function pickFolder(): Promise<string | null> {
  const response = await commands.pickFolder();
  if (response.status === "ok") return response.data;
  toast.error(response.error);
  return null;
}
