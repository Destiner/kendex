import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import ini from "highlight.js/lib/languages/ini";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import markdown from "highlight.js/lib/languages/markdown";
import python from "highlight.js/lib/languages/python";
import typescript from "highlight.js/lib/languages/typescript";
import yaml from "highlight.js/lib/languages/yaml";

// highlight.js's full bundle ships ~190 grammars; catalog content only ever
// needs the handful of languages harness config actually ships in, so the
// core build is used and only these are registered — everything else stays
// out of the app bundle. (ini's built-in "toml" alias covers TOML files.)
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("ini", ini);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("markdown", markdown);
hljs.registerLanguage("python", python);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("yaml", yaml);

const REGISTERED = hljs.listLanguages();

/** File extension → registered hljs language name, via hljs's own alias
 *  table (".sh" -> "bash", ".yml" -> "yaml", ...). null when the extension
 *  isn't one of the languages registered above. */
export function languageFromPath(path: string): string | null {
  const basename = path.split("/").pop() ?? path;
  const ext = basename.includes(".") ? basename.split(".").pop() : null;
  return ext && hljs.getLanguage(ext) ? ext : null;
}

// highlight.js tokenizes `code` as plain text and re-escapes every character
// it emits — it never interprets the input as markup — so the returned HTML
// is safe to render even when the source is an untrusted catalog file.
export function highlightCode(
  code: string,
  language?: string | null,
): { html: string; language: string | null } {
  if (language && hljs.getLanguage(language)) {
    const result = hljs.highlight(code, { language, ignoreIllegals: true });
    return { html: result.value, language };
  }
  const guess = hljs.highlightAuto(code, REGISTERED);
  if (guess.language && guess.relevance > 0) {
    return { html: guess.value, language: guess.language };
  }
  return { html: escapeText(code), language: null };
}

function escapeText(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
