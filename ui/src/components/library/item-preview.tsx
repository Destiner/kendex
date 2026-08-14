import { useEffect, useState } from "react";
import {
  commands,
  type HarnessId,
  type ItemKind,
  type Scope,
} from "@/bindings";
import { Skeleton } from "@/components/ui/skeleton";
import { renderMarkdown } from "@/lib/markdown";

type PreviewState =
  | { status: "loading" }
  | { status: "error"; error: string }
  | { status: "ok"; path: string; content: string; truncated: boolean };

/** The CONTENT section of the item detail panel: fetches the file behind
 *  one installation and renders it — markdown lightly styled, everything
 *  else as a plain code block. */
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

  return (
    <div className="space-y-2">
      {state.truncated ? (
        <p className="text-xs text-muted-foreground">Showing first 64 KB</p>
      ) : null}
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
          // biome-ignore lint/security/noDangerouslySetInnerHtml: renderMarkdown escapes raw HTML tags and unsafe link/image URLs before this ever runs
          dangerouslySetInnerHTML={{ __html: renderMarkdown(state.content) }}
        />
      ) : (
        <pre className="max-h-96 overflow-auto rounded-md border bg-muted/30 p-3 font-mono text-xs">
          <code>{state.content}</code>
        </pre>
      )}
    </div>
  );
}
