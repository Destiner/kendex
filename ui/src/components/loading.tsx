import { useEffect, useState } from "react";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

/**
 * A line of text that hasn't arrived. Sized like the text it stands in for —
 * a title bar is the height of a title — so the page keeps its shape and
 * nothing jumps when the words land.
 */
export function TextBar({
  width,
  title,
  className,
}: {
  /** A Tailwind width class; the caller varies these so a stack of bars
   *  reads as different sentences rather than one repeated one. */
  width: string;
  /** Stand in for a row's title rather than its second line. */
  title?: boolean;
  className?: string;
}) {
  return (
    <Skeleton
      className={cn("rounded-sm", title ? "h-3.5" : "h-3", width, className)}
    />
  );
}

/** A round placeholder — a status dot, an avatar, an icon. */
export function DotBar({ className }: { className?: string }) {
  return <Skeleton className={cn("size-2 shrink-0 rounded-full", className)} />;
}

// Braille eighth-dots, the terminal's own spinner. Ten frames, one revolution.
const FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const FRAME_MS = 80;

/**
 * For a wait with no shape to borrow — a comparison being computed, a file
 * being read — where a placeholder would be inventing a layout it cannot
 * know. Decorative: whatever it sits beside says what is happening.
 */
export function DotSpinner({ className }: { className?: string }) {
  const [frame, setFrame] = useState(0);
  useEffect(() => {
    const tick = setInterval(
      () => setFrame((at) => (at + 1) % FRAMES.length),
      FRAME_MS,
    );
    return () => clearInterval(tick);
  }, []);
  return (
    <span aria-hidden className={cn("font-mono leading-none", className)}>
      {FRAMES[frame]}
    </span>
  );
}
