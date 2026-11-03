import { describe, expect, it } from "vitest";
import { formatRustyDebug } from "./DebugLogCard";

describe("formatRustyDebug", () => {
  it("keeps escaped quotes inside strings from opening structure", () => {
    const raw = 'Foo { a: "x\\\\", b: "{{{{" }';
    expect(formatRustyDebug(raw)).toBe('Foo {\n    a: "x\\\\",\n    b: "{{{{"\n}');
  });

  it("falls back to the raw text when nesting or output would blow up", () => {
    const hostile = `Foo { ${"[".repeat(5000)} }`;
    expect(formatRustyDebug(hostile)).toBe(hostile);
    const huge = `Foo { ${"a, ".repeat(100_000)} }`;
    expect(formatRustyDebug(huge)).toBe(huge);
  });
});
