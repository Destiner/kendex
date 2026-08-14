// Coarse on purpose — the status footer only needs "how stale is this",
// not second-level precision.
export function relativeTime(fromMs: number, toMs: number): string {
  const deltaSec = Math.max(0, Math.round((toMs - fromMs) / 1000));
  if (deltaSec < 60) return "just now";
  const deltaMin = Math.round(deltaSec / 60);
  if (deltaMin < 60) return `${deltaMin}m ago`;
  const deltaHour = Math.round(deltaMin / 60);
  if (deltaHour < 24) return `${deltaHour}h ago`;
  const deltaDay = Math.round(deltaHour / 24);
  return `${deltaDay}d ago`;
}
