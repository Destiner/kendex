import { cn } from "@/lib/utils";

const TONE_CLASSES = {
  good: "bg-good",
  warning: "bg-warning",
  critical: "bg-critical",
  info: "bg-info",
  muted: "bg-muted-foreground",
} as const;

export function StatusDot({
  tone,
  className,
}: {
  tone: keyof typeof TONE_CLASSES;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "size-2 shrink-0 rounded-full",
        TONE_CLASSES[tone],
        className,
      )}
    />
  );
}
