import { Marked, type Tokens } from "marked";
import { highlightCode } from "@/lib/highlight";

// Catalog content is adversarial input: a SKILL.md a person previews here
// was written by whoever published the catalog, not by vstack. marked
// passes raw HTML straight through by default and only percent-encodes
// link/image URLs (it does not reject `javascript:`), so both paths are
// overridden to keep the preview inert — no injected markup, no clickable
// script URLs.
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

const SAFE_HREF = /^(https?:|mailto:|#|\.?\/)/i;
function safeHref(href: string): string | null {
  return SAFE_HREF.test(href.trim()) ? href : null;
}

const renderer = new Marked({ gfm: true });
renderer.use({
  renderer: {
    html({ text }: Tokens.HTML | Tokens.Tag): string {
      return escapeHtml(text);
    },
    link({ href, title, tokens }: Tokens.Link): string {
      const text = this.parser.parseInline(tokens);
      const safe = safeHref(href);
      if (!safe) return text;
      const titleAttr = title ? ` title="${escapeHtml(title)}"` : "";
      return `<a href="${escapeHtml(safe)}"${titleAttr} rel="noopener noreferrer">${text}</a>`;
    },
    image({ href, title, text }: Tokens.Image): string {
      const safe = safeHref(href);
      if (!safe) return escapeHtml(text);
      const titleAttr = title ? ` title="${escapeHtml(title)}"` : "";
      return `<img src="${escapeHtml(safe)}" alt="${escapeHtml(text)}"${titleAttr}>`;
    },
    // `text` here is the fence's raw, unescaped source — exactly what
    // highlightCode expects. highlight.js tokenizes it as plain text and
    // re-escapes what it emits, so a fenced `<script>` still can't inject.
    code({ text, lang }: Tokens.Code): string {
      const requested = lang?.trim().split(/\s+/)[0]?.toLowerCase() || null;
      const { html, language } = highlightCode(text, requested);
      const cls = language ? `hljs language-${language}` : "hljs";
      return `<pre><code class="${cls}">${html}</code></pre>\n`;
    },
  },
});

export function renderMarkdown(source: string): string {
  return renderer.parse(source, { async: false }) as string;
}

// SKILL.md and agent files open with a YAML frontmatter block that the
// preview's own header already surfaces as name/description — left in, it
// renders as a stray "---" rule followed by raw "key: value" text instead
// of prose.
const FRONTMATTER = /^---\r?\n[\s\S]*?\r?\n---\r?\n?/;

export function stripFrontmatter(source: string): string {
  return source.replace(FRONTMATTER, "");
}
