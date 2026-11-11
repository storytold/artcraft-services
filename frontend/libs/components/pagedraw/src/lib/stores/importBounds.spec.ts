import { describe, expect, it } from "vitest";
import { MAX_LINE_POINTS, isValidLineNode } from "./SceneState";
import { MAX_NATIVE_DIMENSION, clampNativeSize } from "../components/Model3DOverlay";

describe("imported scene bounds", () => {
  it("rejects line points that are not a bounded array of finite numbers", () => {
    expect(isValidLineNode({ type: "line", points: [0, 0, 10, 10] })).toBe(true);
    expect(isValidLineNode({ type: "line", points: { length: 1e9 } })).toBe(false);
    expect(isValidLineNode({ type: "line", points: [0, Number.NaN] })).toBe(false);
    expect(
      isValidLineNode({ type: "line", points: new Array(MAX_LINE_POINTS + 2).fill(0) }),
    ).toBe(false);
  });

  it("clamps imported native render dimensions", () => {
    expect(clampNativeSize(1e9, 100)).toBe(MAX_NATIVE_DIMENSION);
    expect(clampNativeSize(Number.NaN, 100)).toBe(100);
    expect(clampNativeSize(-5, 100)).toBe(1);
    expect(clampNativeSize(undefined, Number.POSITIVE_INFINITY)).toBe(1);
  });
});
