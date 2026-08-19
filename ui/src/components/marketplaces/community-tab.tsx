import { Globe, RefreshCw, Star } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { DirectoryRow } from "@/bindings";
import { EmptyState } from "@/components/empty-state";
import { SubscribeDialog } from "@/components/marketplaces/subscribe-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { PAGE_BODY, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { useCommunityStore } from "@/stores/community";
import { useNavStore } from "@/stores/nav";
import { SkillsShSearch } from "./skillssh-search";

const ANY_TAG = "any";

/** The kendex.ai directory plus Skills.sh. The list is served from the
 * app's cache — offline it stays on screen with its "as of" line, never
 * blank. */
export function CommunityTab() {
  const directory = useCommunityStore((s) => s.directory);
  const loading = useCommunityStore((s) => s.loading);
  const error = useCommunityStore((s) => s.error);
  const skillsshAvailable = useCommunityStore((s) => s.skillsshAvailable);
  const load = useCommunityStore((s) => s.load);
  const goToMarketplaces = useNavStore((s) => s.goToMarketplaces);

  const [section, setSection] = useState<"directory" | "skillssh">("directory");
  const [query, setQuery] = useState("");
  const [tag, setTag] = useState(ANY_TAG);
  // Keyed remount so a row's reference lands in the dialog's initial state.
  const [subscribeTo, setSubscribeTo] = useState<string | null>(null);

  useEffect(() => {
    void load(false);
  }, [load]);

  const tags = useMemo(
    () =>
      [...new Set((directory?.rows ?? []).flatMap((row) => row.tags))].sort(),
    [directory],
  );
  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (directory?.rows ?? []).filter(
      (row) =>
        (tag === ANY_TAG || row.tags.includes(tag)) &&
        (!needle ||
          row.name.toLowerCase().includes(needle) ||
          row.repo.toLowerCase().includes(needle) ||
          (row.description ?? "").toLowerCase().includes(needle)),
    );
  }, [directory, query, tag]);

  return (
    <div className={cn(PAGE_BODY, "pt-0")}>
      <div className={cn(WIDE_CONTENT_WIDTH, "space-y-4")}>
        <div className="flex items-center gap-1">
          <Button
            size="sm"
            variant={section === "directory" ? "secondary" : "ghost"}
            onClick={() => setSection("directory")}
          >
            Directory
          </Button>
          {skillsshAvailable ? (
            <Button
              size="sm"
              variant={section === "skillssh" ? "secondary" : "ghost"}
              onClick={() => setSection("skillssh")}
            >
              Skills.sh
            </Button>
          ) : null}
        </div>

        {section === "skillssh" ? (
          <SkillsShSearch onInstall={(url) => setSubscribeTo(url)} />
        ) : error && !directory ? (
          <EmptyState icon={Globe} title="kendex.ai is not reachable">
            {error}
            <div className="mt-3">
              <Button size="sm" variant="outline" onClick={() => load(true)}>
                Try again
              </Button>
            </div>
          </EmptyState>
        ) : (
          <>
            <div className="flex items-center gap-2">
              <Input
                className="max-w-xs"
                placeholder="Search marketplaces"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              {tags.length > 0 ? (
                <Select value={tag} onValueChange={(v) => setTag(v ?? ANY_TAG)}>
                  <SelectTrigger className="w-36">
                    <SelectValue>
                      {(current: string) =>
                        current === ANY_TAG ? "Any tag" : current
                      }
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={ANY_TAG}>Any tag</SelectItem>
                    {tags.map((t) => (
                      <SelectItem key={t} value={t}>
                        {t}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              ) : null}
              <div className="ml-auto flex items-center gap-1 text-xs text-muted-foreground">
                {directory
                  ? directory.stale
                    ? `Shown as of ${dayOf(directory.fetchedAt)} — kendex.ai is not reachable`
                    : `Updated ${agoLabel(directory.fetchedAt)}`
                  : null}
                <Button
                  size="icon-sm"
                  variant="ghost"
                  aria-label="Refresh the directory"
                  disabled={loading}
                  onClick={() => load(true)}
                >
                  <RefreshCw
                    className={cn("size-3.5", loading && "animate-spin")}
                  />
                </Button>
              </div>
            </div>

            {directory && rows.length === 0 ? (
              <p className="t-desc py-8 text-center text-sm text-muted-foreground">
                {directory.rows.length === 0
                  ? "The directory has no listed marketplaces yet."
                  : "No listed marketplace matches this search."}
              </p>
            ) : (
              <div className="divide-y rounded-lg border">
                {rows.map((row) => (
                  <DirectoryRowLine
                    key={row.repo}
                    row={row}
                    onSubscribe={() => setSubscribeTo(row.repo)}
                  />
                ))}
              </div>
            )}
            <p className="text-xs text-muted-foreground">
              Not listed here?{" "}
              <button
                type="button"
                className="underline underline-offset-2 hover:text-foreground"
                onClick={() => goToMarketplaces("mine")}
              >
                Submit your own marketplace
              </button>
            </p>
          </>
        )}
      </div>
      {subscribeTo !== null ? (
        <SubscribeDialog
          key={subscribeTo}
          open
          onOpenChange={(open) => {
            if (!open) setSubscribeTo(null);
          }}
          initialReference={subscribeTo}
        />
      ) : null}
    </div>
  );
}

function DirectoryRowLine({
  row,
  onSubscribe,
}: {
  row: DirectoryRow;
  onSubscribe: () => void;
}) {
  return (
    <div className="flex items-center gap-3 px-4 py-3">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{row.name}</span>
          {row.featured ? (
            <Badge variant="secondary" className="gap-1">
              <Star className="size-3" /> featured
            </Badge>
          ) : null}
        </div>
        <div className="flex items-center gap-2">
          <a
            className="truncate font-mono text-xs text-muted-foreground underline-offset-2 hover:underline"
            href={`https://github.com/${row.repo}`}
            target="_blank"
            rel="noreferrer"
          >
            {row.repo}
          </a>
          {row.description ? (
            <span className="truncate text-xs text-muted-foreground">
              {row.description}
            </span>
          ) : null}
        </div>
      </div>
      <span className="shrink-0 text-xs text-muted-foreground">
        {row.packageCount} {row.packageCount === 1 ? "pkg" : "pkgs"}
        {row.bundleCount > 0 ? ` · ${row.bundleCount} bundles` : ""}
      </span>
      {row.subscribed ? (
        <span className="shrink-0 text-xs text-muted-foreground">
          Subscribed ✓
        </span>
      ) : (
        <Button size="sm" variant="outline" onClick={onSubscribe}>
          Subscribe
        </Button>
      )}
    </div>
  );
}

function agoLabel(iso: string): string {
  const seconds = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
  if (seconds < 90) return "just now";
  if (seconds < 5400) return `${Math.round(seconds / 60)}m ago`;
  if (seconds < 129_600) return `${Math.round(seconds / 3600)}h ago`;
  return `${Math.round(seconds / 86_400)}d ago`;
}

function dayOf(iso: string): string {
  return iso.slice(0, 10);
}
