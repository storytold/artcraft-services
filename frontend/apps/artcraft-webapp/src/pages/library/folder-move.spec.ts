import { describe, expect, it } from "vitest";
import { getFolderMoveDestinations, getFolderPath } from "./folder-move";

const folders = [
  { id: "a", name: "Projects", parentId: null },
  { id: "b", name: "Images", parentId: "a" },
  { id: "c", name: "Drafts", parentId: "b" },
  { id: "d", name: "Images", parentId: null },
];

describe("folder move destinations", () => {
  it("excludes the complete source subtree but allows ancestors and other branches", () => {
    expect(getFolderMoveDestinations(folders, "b").map((f) => f.id)).toEqual([
      "a",
      "d",
    ]);
    expect(getFolderMoveDestinations(folders, "a").map((f) => f.id)).toEqual([
      "d",
    ]);
  });

  it("distinguishes duplicate names using their full paths", () => {
    const byId = new Map(folders.map((f) => [f.id, f]));
    expect(getFolderPath(folders[1], byId)).toBe("Projects / Images");
    expect(getFolderPath(folders[3], byId)).toBe("Images");
  });

  it("terminates even if existing folder data contains a cycle", () => {
    const cyclic = [{ ...folders[0], parentId: "c" }, ...folders.slice(1)];
    expect(getFolderMoveDestinations(cyclic, "a").map((f) => f.id)).toEqual([
      "d",
    ]);
  });
});
