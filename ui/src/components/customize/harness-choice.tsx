import type { HarnessId } from "@/bindings";
import { HarnessIcon } from "@/components/harness-icon";
import { harnessName } from "@/lib/labels";

/** Which harnesses this hook installs for; none chosen means all of them.
 *  A dimmed mark is a harness this hook skips. */
export function HarnessChoice({
  all,
  chosen,
  onChoose,
}: {
  all: HarnessId[];
  chosen: string[] | null;
  onChoose: (harnesses: string[] | null) => void;
}) {
  const active = (harness: HarnessId) =>
    chosen === null || chosen.includes(harness);
  const toggle = (harness: HarnessId) => {
    const next = all.filter((h) => (h === harness ? !active(h) : active(h)));
    onChoose(next.length === all.length ? null : next);
  };
  return (
    <div className="flex h-9 items-center gap-1">
      {all.map((harness) => (
        <button
          key={harness}
          type="button"
          aria-label={harnessName(harness)}
          aria-pressed={active(harness)}
          title={harnessName(harness)}
          onClick={() => toggle(harness)}
          className={
            active(harness)
              ? "rounded-md border border-border p-1.5"
              : "rounded-md border border-transparent p-1.5 opacity-30 hover:opacity-60"
          }
        >
          <HarnessIcon harness={harness} className="size-4" />
        </button>
      ))}
    </div>
  );
}
