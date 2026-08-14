import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./markdown";

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
});
