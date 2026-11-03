import { describe, expect, it } from "vitest";
import { sanitizeGridSpacing } from "./canvasBridge";

describe("sanitizeGridSpacing", () => {
  it("keeps valid spacing", () => {
    expect(sanitizeGridSpacing(24)).toBe(24);
  });

  it("replaces spacing that would break the background grid", () => {
    for (const bad of [0, -32, NaN, Infinity, 1e9, "32", null, undefined]) {
      expect(sanitizeGridSpacing(bad)).toBe(32);
    }
  });
});
