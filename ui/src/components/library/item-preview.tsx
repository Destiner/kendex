import { Copy } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import {
  commands,
  type HarnessId,
  type ItemKind,
  type Scope,
} from "@/bindings";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { highlightCode, languageFromPath } from "@/lib/highlight";
import { renderMarkdown, stripFrontmatter } from "@/lib/markdown";

type PreviewState =
  | { status: "loading" }
  | { status: "error"; error: string }
  | { status: "ok"; path: string; content: string; truncated: boolean };

/** The CONTENT section of the item detail flyout: fetches the file behind
 *  one installation and renders it — markdown lightly styled with
 *  highlighted fences, everything else as syntax-highlighted code. Both
 *  sit inside one inset box with a sticky basename/copy sub-header; the
 *  flyout itself is the only scroll container, so this never scrolls on
 *  its own. */
export function ItemPreview({
  scope,
  kind,
  name,
  harness,
}: {
  scope: Scope;
  kind: ItemKind;
  name: string;
  harness: HarnessId;
}) {
  const [state, setState] = useState<PreviewState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });
    void commands.itemSource(scope, kind, name, harness).then((response) => {
      if (cancelled) return;
      setState(
        response.status === "ok"
          ? { status: "ok", ...response.data }
          : { status: "error", error: response.error },
      );
    });
    return () => {
      cancelled = true;
    };
  }, [scope, kind, name, harness]);

  if (state.status === "loading") {
    return (
      <div className="space-y-2">
        <Skeleton className="h-3.5 w-3/4" />
        <Skeleton className="h-3.5 w-full" />
        <Skeleton className="h-3.5 w-5/6" />
        <Skeleton className="h-3.5 w-2/3" />
      </div>
    );
  }

  if (state.status === "error") {
    return (
      <p className="text-sm text-muted-foreground">
        Couldn't load this file: {state.error}
      </p>
    );
  }

  const isMarkdown = state.path.toLowerCase().endsWith(".md");
  const basename = state.path.split("/").pop() ?? state.path;

  const copyPath = () => {
    void navigator.clipboard.writeText(state.path).then(() => {
      toast.success("Path copied");
    });
  };

  return (
    <div className="rounded-lg border bg-muted/20">
      <div className="sticky top-0 z-10 flex items-center justify-between gap-2 rounded-t-lg border-b bg-muted/60 px-3 py-1.5 backdrop-blur-sm">
        <span className="min-w-0 truncate font-mono text-xs text-muted-foreground">
          {basename}
        </span>
        <span className="flex shrink-0 items-center gap-2">
          {state.truncated ? (
            <span className="text-[11px] text-muted-foreground">
              Showing first 64 KB
            </span>
          ) : null}
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label="Copy path"
            title="Copy path"
            onClick={copyPath}
          >
            <Copy className="size-3.5" />
          </Button>
        </span>
      </div>
      <div className="p-3">
        {isMarkdown ? (
          // A preview link that navigated the app window would be worse than
          // no link at all — there's no safe "open externally" command wired
          // up here, so clicks are swallowed instead of following the href.
          // This is a click guard on rendered content, not an interactive
          // control, so it has no keyboard equivalent to wire up.
          // biome-ignore lint/a11y/noStaticElementInteractions: swallows clicks bubbling from links inside untrusted rendered markdown, not a widget
          // biome-ignore lint/a11y/useKeyWithClickEvents: same — nothing here to reach by keyboard
          <div
            className="prose-preview max-w-none text-sm"
            onClick={(event) => {
              if ((event.target as HTMLElement).closest("a"))
                event.preventDefault();
            }}
            // biome-ignore lint/security/noDangerouslySetInnerHtml: renderMarkdown escapes raw HTML tags and unsafe link/image URLs, and highlights fenced code from plain text, before this ever runs
            dangerouslySetInnerHTML={{
              __html: renderMarkdown(stripFrontmatter(state.content)),
            }}
          />
        ) : (
          <CodeBlock path={state.path} content={state.content} />
        )}
      </div>
    </div>
  );
}

// highlight.js tokenizes `content` as plain text and re-escapes what it
// emits — it never interprets the file's own bytes as markup — so this is
// as safe as the plain <pre><code>{content}</code> it replaces.
function CodeBlock({ path, content }: { path: string; content: string }) {
  const { html, language } = highlightCode(content, languageFromPath(path));
  const cls = language ? `hljs language-${language}` : "hljs";
  return (
    <pre className="overflow-x-auto font-mono text-xs">
      {/* biome-ignore lint/security/noDangerouslySetInnerHtml: highlightCode escapes every character it emits (see highlight.ts) */}
      <code className={cls} dangerouslySetInnerHTML={{ __html: html }} />
    </pre>
  );
}
