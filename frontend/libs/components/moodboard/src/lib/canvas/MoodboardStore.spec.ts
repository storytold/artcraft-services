import { beforeEach, describe, expect, it } from "vitest";
import { useMoodboardStore } from "./MoodboardStore";
import type { GroupNode, MoodboardNode } from "./types";

function group(id: string, childIds: string[]): GroupNode {
  return {
    id,
    kind: "group",
    parentId: null,
    x: 0,
    y: 0,
    width: 10,
    height: 10,
    rotation: 0,
    zIndex: 0,
    childIds,
  };
}

describe("deleteSelected", () => {
  beforeEach(() => {
    useMoodboardStore.setState({ nodes: {}, rootOrder: [], selectedIds: new Set() });
  });

  it("finishes quickly on a deep chain of shared descendants", () => {
    // Each level points at the next level twice, so a walk that does not
    // skip visited nodes does 2^depth work.
    const depth = 64;
    const nodes: Record<string, MoodboardNode> = {};
    for (let i = 0; i < depth; i++) {
      const next = `n${i + 1}`;
      nodes[`n${i}`] = group(`n${i}`, i + 1 < depth ? [next, next] : []);
    }
    nodes["keep"] = group("keep", []);
    useMoodboardStore.setState({
      nodes,
      rootOrder: ["n0", "keep"],
      selectedIds: new Set(["n0"]),
    });

    const started = Date.now();
    useMoodboardStore.getState().deleteSelected();
    expect(Date.now() - started).toBeLessThan(1000);

    const state = useMoodboardStore.getState();
    expect(Object.keys(state.nodes)).toEqual(["keep"]);
    expect(state.rootOrder).toEqual(["keep"]);
  });

  it("terminates on cyclic container references", () => {
    useMoodboardStore.setState({
      nodes: { a: group("a", ["b"]), b: group("b", ["a"]) },
      rootOrder: ["a"],
      selectedIds: new Set(["a"]),
    });
    useMoodboardStore.getState().deleteSelected();
    expect(useMoodboardStore.getState().nodes).toEqual({});
  });
});
