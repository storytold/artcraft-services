import { describe, expect, it } from "vitest";
import { sanitizeCssColor } from "./cssColor";

describe("sanitizeCssColor", () => {
  it("accepts plain color values", () => {
    expect(sanitizeCssColor("rgb(96, 165, 250)")).toBe("rgb(96, 165, 250)");
    expect(sanitizeCssColor("rgba(0 0 0 / 50%)")).toBe("rgba(0 0 0 / 50%)");
    expect(sanitizeCssColor("hsl(200, 50%, 40%)")).toBe("hsl(200, 50%, 40%)");
    expect(sanitizeCssColor("#ff00aa")).toBe("#ff00aa");
    expect(sanitizeCssColor("red")).toBe("red");
  });

  it("rejects values that could break out of a style attribute", () => {
    expect(sanitizeCssColor('red"><img src=x onerror=alert(1)>')).toBeUndefined();
    expect(sanitizeCssColor("red;background:url(https://x)")).toBeUndefined();
    expect(sanitizeCssColor("var(--x)")).toBeUndefined();
    expect(sanitizeCssColor("rgb(1,2,3)\"")).toBeUndefined();
    expect(sanitizeCssColor(42)).toBeUndefined();
  });
});
