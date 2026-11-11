import { describe, expect, it } from "vitest";
import { sanitizeGridSpacing } from "./canvasBridge";

describe("sanitizeGridSpacing", () => {
  it("keeps valid spacing", () => {
    expect(sanitizeGridSpacing(24)).toBe(24);
    expect(sanitizeGridSpacing(8192)).toBe(8192);
    expect(sanitizeGridSpacing(1e9)).toBe(1e9);
  });

  it("replaces spacing that would break the background grid", () => {
    for (const bad of [0, -32, 0.5, NaN, Infinity, "32", null, undefined]) {
      expect(sanitizeGridSpacing(bad)).toBe(32);
    }
  });
});
