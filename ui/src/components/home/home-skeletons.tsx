import { DotBar, TextBar } from "@/components/loading";
import { Skeleton } from "@/components/ui/skeleton";

// Each entry is one row's two bars — a headline and the sentence under it.
// Varied on purpose: three identical pairs read as a loading graphic, three
// different ones read as a list that hasn't arrived.
const ATTENTION_ROWS = [
  ["w-44", "w-72"],
  ["w-36", "w-64"],
];

/** The shape of `AttentionSection`, before the audit has said anything.
 *  Same card, same rows, same paddings — the words are what's missing. */
export function AttentionSkeleton() {
  return (
    <div className="divide-y overflow-hidden rounded-xl border bg-card">
      {ATTENTION_ROWS.map(([title, detail]) => (
        <div key={title} className="flex items-center gap-3 px-4 py-3.5">
          <DotBar className="mt-1.5 self-start" />
          <span className="flex min-w-0 flex-1 flex-col gap-2">
            <TextBar title width={title} />
            <TextBar width={detail} />
          </span>
          <TextBar width="w-16" />
        </div>
      ))}
    </div>
  );
}

const RECENT_ROWS = ["w-40", "w-28", "w-48", "w-32", "w-36", "w-24"];

/** The shape of `RecentActivity`: an icon, a name, what it is, when. */
export function RecentSkeleton() {
  return (
    <div className="flex flex-col">
      {RECENT_ROWS.map((name) => (
        <div key={name} className="flex items-center gap-3 px-2 py-2.5">
          <Skeleton className="size-4 shrink-0 rounded" />
          <TextBar title width={name} />
          <span className="flex-1" />
          <TextBar width="w-28" className="hidden sm:block" />
          <TextBar width="w-12" />
        </div>
      ))}
    </div>
  );
}

/** The shape of the three stat tiles: a number over its label. */
export function StatsSkeleton() {
  return (
    <div className="grid grid-cols-3 gap-3">
      {["one", "two", "three"].map((tile) => (
        <div
          key={tile}
          className="flex flex-col gap-2 rounded-lg border px-4 py-3"
        >
          <Skeleton className="h-6 w-10 rounded-sm" />
          <TextBar width="w-16" />
        </div>
      ))}
    </div>
  );
}
