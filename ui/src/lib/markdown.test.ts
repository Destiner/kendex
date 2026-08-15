import { describe, expect, it } from "vitest";
import { renderMarkdown, stripFrontmatter } from "./markdown";

describe("renderMarkdown", () => {
  it("renders ordinary markdown", () => {
    const html = renderMarkdown("# Title\n\nSome **bold** text.");
    expect(html).toContain("<h1>Title</h1>");
    expect(html).toContain("<strong>bold</strong>");
  });

  it("escapes raw HTML instead of passing it through", () => {
    const html = renderMarkdown('<script>alert("hi")</script>\n\nText after.');
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
  });

  it("drops javascript: links but keeps their text", () => {
    const html = renderMarkdown("[click me](javascript:alert(1))");
    expect(html).not.toContain("javascript:");
    expect(html).toContain("click me");
    expect(html).not.toContain("<a ");
  });

  it("keeps http(s) and mailto links", () => {
    const html = renderMarkdown("[docs](https://example.com/skill)");
    expect(html).toContain('href="https://example.com/skill"');
    expect(html).toContain('rel="noopener noreferrer"');
  });

  it("drops javascript: image sources but keeps the alt text", () => {
    const html = renderMarkdown("![alt](javascript:alert(1))");
    expect(html).not.toContain("<img");
    expect(html).toContain("alt");
  });

  it("highlights a fenced code block and still escapes its markup", () => {
    const source = '```js\nconst x = "<script>alert(1)</script>";\n```';
    const html = renderMarkdown(source);
    expect(html).not.toContain("<script>alert");
    expect(html).toContain("&lt;script&gt;");
    expect(html).toContain('class="hljs language-js"');
    expect(html).toContain("hljs-keyword");
  });

  it("falls back to escaped, unhighlighted text when nothing registered matches", () => {
    const html = renderMarkdown(
      "```rust\nlorem ipsum dolor sit amet consectetur\n```",
    );
    expect(html).toContain("lorem ipsum dolor sit amet consectetur");
    expect(html).toContain('class="hljs"');
    expect(html).not.toContain("language-");
  });
});

describe("stripFrontmatter", () => {
  it("leaves content with no frontmatter untouched", () => {
    expect(stripFrontmatter("# Title\n\nBody.")).toBe("# Title\n\nBody.");
  });

  it("removes a terminated frontmatter block", () => {
    const source = "---\nname: deploy\ndescription: ships it\n---\n# Title\n";
    expect(stripFrontmatter(source)).toBe("# Title\n");
  });

  it("leaves an unterminated block alone rather than eating the rest", () => {
    const source = "---\nname: deploy\n\n# Title\n";
    expect(stripFrontmatter(source)).toBe(source);
  });
});
