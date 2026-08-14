import { Minus, Square, X } from "lucide-react";
import { commands } from "@/bindings";
import { Button } from "@/components/ui/button";
import { WINDOW_CONTROL_LABELS } from "@/lib/labels";

// The frameless window has no OS titlebar, so this bar both provides the
// drag handle and hosts the window controls it replaced. data-tauri-drag-region
// only affects the element it's on — the buttons inside stay clickable
// because they don't carry it themselves.
export function Titlebar() {
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: double-click here is a convenience alias for the maximize button already in the bar
    <div
      data-tauri-drag-region
      onDoubleClick={() => void commands.windowToggleMaximize()}
      className="flex h-8 shrink-0 items-center justify-end border-b bg-background"
    >
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={WINDOW_CONTROL_LABELS.minimize}
        title={WINDOW_CONTROL_LABELS.minimize}
        onClick={() => void commands.windowMinimize()}
      >
        <Minus className="size-3.5" />
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={WINDOW_CONTROL_LABELS.maximize}
        title={WINDOW_CONTROL_LABELS.maximize}
        onClick={() => void commands.windowToggleMaximize()}
      >
        <Square className="size-3.5" />
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={WINDOW_CONTROL_LABELS.close}
        title={WINDOW_CONTROL_LABELS.close}
        onClick={() => void commands.windowClose()}
        className="hover:bg-destructive hover:text-white"
      >
        <X className="size-3.5" />
      </Button>
    </div>
  );
}
