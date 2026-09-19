import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DragEvent } from "react";
import { useFolderDrag } from "./use-folder-drag";

const { moveFolder } = vi.hoisted(() => ({ moveFolder: vi.fn() }));
vi.mock("./library-folders-store", () => ({
  useLibraryFoldersStore: {
    getState: () => ({
      folders: [
        { id: "source", name: "Source", parentId: "parent" },
        { id: "child", name: "Child", parentId: "source" },
        { id: "target", name: "Target", parentId: null },
        { id: "parent", name: "Parent", parentId: null },
      ],
      moveFolder,
    }),
  },
}));

afterEach(() => {
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

describe("folder dragging across grid and sidebar", () => {
  it.each(["target", null])("moves to destination %s", (destination) => {
    const grid = renderHook(useFolderDrag).result.current;
    const sidebar = renderHook(useFolderDrag).result.current;
    grid.onDragStart(dragEvent("source"));
    const drop = dragEvent(destination);
    sidebar.onDragOver(drop);
    expect(drop.preventDefault).toHaveBeenCalled();
    expect((drop.target as HTMLElement).classList.contains("folder-drag-over")).toBe(true);
    sidebar.onDrop(drop);
    expect(moveFolder).toHaveBeenCalledWith("source", destination);
    expect(document.querySelector(".folder-drag-over")).toBeNull();
    grid.onDragEnd();
  });

  it.each(["source", "child", "parent"])("rejects invalid or unchanged destination %s", (destination) => {
    const drag = renderHook(useFolderDrag).result.current;
    drag.onDragStart(dragEvent("source"));
    const drop = dragEvent(destination);
    drag.onDragOver(drop);
    drag.onDrop(drop);
    expect(drop.preventDefault).not.toHaveBeenCalled();
    expect(moveFolder).not.toHaveBeenCalled();
    drag.onDragEnd();
  });

  it("does not accept drops after cancellation", () => {
    const drag = renderHook(useFolderDrag).result.current;
    drag.onDragStart(dragEvent("source"));
    drag.onDragEnd();
    drag.onDrop(dragEvent("target"));
    expect(moveFolder).not.toHaveBeenCalled();
  });
});

function dragEvent(id: string | null): DragEvent<HTMLElement> {
  const container = document.createElement("div");
  const target = document.createElement("button");
  target.draggable = true;
  if (id) target.dataset.folderId = id;
  else target.dataset.folderRoot = "";
  container.append(target);
  document.body.append(container);
  return {
    target,
    currentTarget: container,
    dataTransfer: { setData: vi.fn() },
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as unknown as DragEvent<HTMLElement>;
}
