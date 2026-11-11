import { describe, expect, it } from "vitest";
import { markdownToHtml, safeUrl } from "./markdown";

function parse(html: string): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = html;
  return root;
}

describe("markdownToHtml", () => {
  it("still renders the markdown it supports", () => {
    const html = markdownToHtml(
      [
        "## Heading",
        "Some **bold** and *italic* text with a [link](https://example.com/a?b=1&c=2).",
        "",
        "- item",
        "> quote",
        "",
        "![Alt text](./images/a.png|300)",
        "@youtube(pGn-1BKo3nY)",
        "@loop_autoplay(https://cdn.example.com/a.mp4)",
      ].join("\n"),
      "/news/",
    );
    const root = parse(html);
    expect(root.querySelector("h2")?.textContent).toBe("Heading");
    expect(root.querySelector("strong")?.textContent).toBe("bold");
    expect(root.querySelector("em")?.textContent).toBe("italic");
    expect(root.querySelector("a")?.getAttribute("href")).toBe(
      "https://example.com/a?b=1&c=2",
    );
    expect(root.querySelector("li")?.textContent).toBe("item");
    expect(root.querySelector("blockquote")?.textContent).toBe("quote");
    const img = root.querySelector("img");
    expect(img?.getAttribute("src")).toBe("/news/images/a.png");
    expect(img?.getAttribute("alt")).toBe("Alt text");
    expect(img?.getAttribute("width")).toBe("300");
    expect(root.querySelector("iframe")?.getAttribute("src")).toBe(
      "https://www.youtube.com/embed/pGn-1BKo3nY",
    );
    expect(root.querySelector("video source")?.getAttribute("src")).toBe(
      "https://cdn.example.com/a.mp4",
    );
  });

  it("renders raw HTML in the source as text", () => {
    const root = parse(
      markdownToHtml(
        [
          '<img src=x onerror="alert(1)">',
          "## <script>alert(1)</script>",
          "- <svg onload=alert(1)>",
          "> <iframe src=javascript:alert(1)>",
        ].join("\n"),
      ),
    );
    expect(root.querySelector("script, svg, iframe, [onerror], [onload]")).toBeNull();
    expect(root.querySelectorAll("img").length).toBe(0);
    expect(root.textContent).toContain('<img src=x onerror="alert(1)">');
  });

  it("does not let link or image text break out of attributes", () => {
    const root = parse(
      markdownToHtml(
        [
          '[x](https://a.example/" onmouseover="alert(1))',
          '![a" onerror="alert(1)](./x.png)',
          '@image(./x.png, 100px" onload="alert(1))',
          "@video(./v.mp4\" onerror=\"alert(1))",
        ].join("\n"),
      ),
    );
    expect(root.querySelector("[onmouseover], [onerror], [onload]")).toBeNull();
  });

  it("neutralizes script URLs in links and media", () => {
    const root = parse(
      markdownToHtml(
        [
          "[click](javascript:alert(1)//)",
          "[click](JaVa\tScRiPt:alert(1)//)",
          "![x](data:text/html,hi)",
          "@video(javascript:alert(1)//.mp4)",
        ].join("\n"),
      ),
    );
    for (const el of Array.from(root.querySelectorAll("a, img, source"))) {
      const url = el.getAttribute("href") ?? el.getAttribute("src") ?? "";
      expect(url).toBe("#");
    }
  });
});

describe("safeUrl", () => {
  it("keeps relative, http(s) and mailto URLs", () => {
    for (const url of ["/a", "./a", "#x", "https://a.example", "http://a", "mailto:a@b.c"]) {
      expect(safeUrl(url)).toBe(url);
    }
  });

  it("rejects other schemes", () => {
    for (const url of ["javascript:alert(1)", " data:text/html,x", "vbscript:x", "java\nscript:x"]) {
      expect(safeUrl(url)).toBe("#");
    }
  });
});
