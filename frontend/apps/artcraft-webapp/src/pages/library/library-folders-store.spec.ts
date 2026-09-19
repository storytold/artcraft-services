import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLibraryFoldersStore } from "./library-folders-store";

const { add, remove, error } = vi.hoisted(() => ({
  add: vi.fn(),
  remove: vi.fn(),
  error: vi.fn(),
}));
vi.mock("@storyteller/api", () => ({
  FoldersApi: class {
    AddSubfolders = add;
    RemoveSubfolders = remove;
  },
  MediaFilesApi: class {},
}));
vi.mock("@storyteller/ui-gallery-modal", () => ({}));
vi.mock("./library-media-map", () => ({ errMsg: String }));
vi.mock("./library-tags-store", () => ({}));
vi.mock("../../components/toast/toast", () => ({
  toast: { error, success: vi.fn() },
}));

describe("moveFolder", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useLibraryFoldersStore.setState({
      folders: [
        { id: "source", name: "Source", parentId: null },
        { id: "child", name: "Child", parentId: "source" },
        { id: "target", name: "Target", parentId: null },
      ],
      folderMediaItems: { source: [], child: [] },
    });
  });

  it("reparents only the source and preserves descendants and cached media", async () => {
    add.mockResolvedValue({ success: true, data: ["source"] });
    const media = useLibraryFoldersStore.getState().folderMediaItems;
    expect(
      await useLibraryFoldersStore.getState().moveFolder("source", "target"),
    ).toBe(true);
    expect(add).toHaveBeenCalledWith({
      folderToken: "target",
      subfolderTokens: ["source"],
    });
    const state = useLibraryFoldersStore.getState();
    expect(state.folders[0].parentId).toBe("target");
    expect(state.folders[1].parentId).toBe("source");
    expect(state.folderMediaItems).toBe(media);
  });

  it("moves a nested folder to the top level", async () => {
    remove.mockResolvedValue({ success: true, data: 1 });
    expect(
      await useLibraryFoldersStore.getState().moveFolder("child", null),
    ).toBe(true);
    expect(remove).toHaveBeenCalledWith({
      folderToken: "source",
      subfolderTokens: ["child"],
    });
    expect(useLibraryFoldersStore.getState().folders[1].parentId).toBeNull();
  });

  it("rejects moves into the source subtree before calling the API", async () => {
    for (const target of ["source", "child"]) {
      expect(
        await useLibraryFoldersStore.getState().moveFolder("source", target),
      ).toBe(false);
    }
    expect(add).not.toHaveBeenCalled();
  });

  it.each([{ success: false }, { success: true, data: [] }])(
    "preserves the tree when the API does not accept the move: %j",
    async (response) => {
      add.mockResolvedValue(response);
      const folders = useLibraryFoldersStore.getState().folders;
      expect(
        await useLibraryFoldersStore.getState().moveFolder("source", "target"),
      ).toBe(false);
      expect(useLibraryFoldersStore.getState().folders).toBe(folders);
      expect(error).toHaveBeenCalled();
    },
  );
});
