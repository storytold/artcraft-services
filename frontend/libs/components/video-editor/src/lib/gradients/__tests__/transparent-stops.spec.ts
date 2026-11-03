import { describe, expect, test } from "vitest";

import { fixTransparentStops } from "../canvas";

const stop = (color: string) => ({ color, offset: null });

describe("fixTransparentStops", () => {
  test("borrows the previous opaque colour, then the next one", () => {
    const result = fixTransparentStops({
      stops: [
        stop("transparent"),
        stop("#ff0000"),
        stop("transparent"),
        stop("rgb(0, 0, 255)"),
        stop("transparent"),
      ],
    });

    expect(result.map((s) => s.color)).toEqual([
      "rgba(255,0,0,0)",
      "#ff0000",
      "rgba(255,0,0,0)",
      "rgb(0, 0, 255)",
      "rgba(0,0,255,0)",
    ]);
  });

  test("skips opaque stops it cannot parse when looking for a donor", () => {
    const result = fixTransparentStops({
      stops: [stop("#00ff00"), stop("hsl(0, 0%, 0%)"), stop("transparent")],
    });

    expect(result[2].color).toBe("rgba(0,255,0,0)");
  });

  test("leaves an all-transparent list unchanged without stalling", () => {
    const stops = Array.from({ length: 200_000 }, () => stop("transparent"));

    const startedAt = performance.now();
    const result = fixTransparentStops({ stops });
    const elapsedMs = performance.now() - startedAt;

    expect(result).toHaveLength(stops.length);
    expect(result.every((s) => s.color === "transparent")).toBe(true);
    // The old per-stop search did ~2e10 comparisons here.
    expect(elapsedMs).toBeLessThan(2_000);
  });
});
