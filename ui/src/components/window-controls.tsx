import { Minus, Square, X } from "lucide-react";
import { commands } from "@/bindings";
import { Button } from "@/components/ui/button";
import { WINDOW_CONTROL_LABELS } from "@/lib/copy";
import { cn } from "@/lib/utils";

// Floats over the content pane's top-right corner, above the drag strip
// beneath it — normal hit-testing gives clicks to whichever element is on
// top, so the buttons stay clickable without needing a drag-region cutout.
export function WindowControls({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center", className)}>
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
