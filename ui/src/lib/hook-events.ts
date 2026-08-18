import type { HookEvent } from "@/bindings";

/**
 * The events matching what someone has typed, over both the name and what
 * the event fires on: a person hunting for "before a command runs" should
 * land on PreToolUse without knowing kendex calls it that.
 */
export function matchingEvents(
  events: HookEvent[],
  query: string,
): HookEvent[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return events;
  return events.filter(
    (event) =>
      event.name.toLowerCase().includes(needle) ||
      event.fires.toLowerCase().includes(needle),
  );
}
