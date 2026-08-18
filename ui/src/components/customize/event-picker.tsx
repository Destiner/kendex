import { Check, ChevronDown, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { HookEvent } from "@/bindings";
import { Input } from "@/components/ui/input";
import { NO_EVENT_MATCHES, PICK_EVENT } from "@/lib/copy-customize";
import { matchingEvents } from "@/lib/hook-events";
import { cn } from "@/lib/utils";

/**
 * Which event a hook listens for, picked from the list the harnesses
 * actually fire rather than typed. A typed event installs cleanly and then
 * never runs — the name is only ever compared, never guessed at — so the
 * only honest field here is a closed list, and it is long enough to want
 * filtering.
 */
export function EventPicker({
  value,
  events,
  onPick,
}: {
  value: string;
  events: HookEvent[];
  onPick: (event: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const away = (event: MouseEvent) => {
      if (!box.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", away);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", away);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const shown = matchingEvents(events, query);

  return (
    <div ref={box} className="relative">
      <button
        type="button"
        aria-label="Event"
        onClick={() => {
          setQuery("");
          setOpen((was) => !was);
        }}
        className="flex h-9 w-full items-center justify-between gap-2 rounded-md border border-input bg-transparent px-3 text-sm transition-colors hover:border-ring"
      >
        <span className={cn(!value && "text-muted-foreground")}>
          {value || PICK_EVENT}
        </span>
        <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
      </button>
      {open ? (
        <div className="absolute z-50 mt-1 flex w-full flex-col overflow-hidden rounded-md border bg-popover shadow-md">
          <div className="relative border-b">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              autoFocus
              aria-label="Filter events"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              className="h-9 rounded-none border-0 pl-8 focus-visible:ring-0"
            />
          </div>
          <div className="max-h-64 overflow-y-auto py-1">
            {shown.length === 0 ? (
              <p className="px-3 py-2 text-[13px] text-muted-foreground">
                {NO_EVENT_MATCHES}
              </p>
            ) : null}
            {shown.map((event) => (
              <button
                key={event.name}
                type="button"
                onClick={() => {
                  onPick(event.name);
                  setOpen(false);
                }}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-accent"
              >
                <Check
                  className={cn(
                    "size-3.5 shrink-0",
                    event.name === value ? "opacity-100" : "opacity-0",
                  )}
                />
                <span className="min-w-0 flex-1">
                  <span className="block text-sm">{event.name}</span>
                  <span className="block text-xs text-muted-foreground">
                    {event.fires}
                  </span>
                </span>
              </button>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}
