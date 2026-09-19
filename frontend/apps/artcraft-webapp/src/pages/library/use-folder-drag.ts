import { useMemo } from "react";
import type { DragEvent } from "react";
import { getFolderMoveDestinations } from "./folder-move";
import { useLibraryFoldersStore } from "./library-folders-store";

const FOLDER_MIME = "application/x-artcraft-folder";
let draggedFolderId: string | null = null;

/** Shared native drag handlers for the library grid and sidebar. */
export function useFolderDrag() {
  return useMemo(() => {
    const clearHighlight = () => {
      document.querySelectorAll(".folder-drag-over").forEach((el) => {
        el.classList.remove("folder-drag-over");
      });
    };
    const targetAt = (event: DragEvent<HTMLElement>) => {
      const target = (event.target as HTMLElement).closest<HTMLElement>(
        "[data-folder-id], [data-folder-root]",
      );
      if (!target || !event.currentTarget.contains(target)) return null;
      return target;
    };
    const canDrop = (target: HTMLElement | null) => {
      if (!draggedFolderId || !target) return false;
      const { folders } = useLibraryFoldersStore.getState();
      const source = folders.find((f) => f.id === draggedFolderId);
      const parentId = target.dataset.folderId ?? null;
      return (
        !!source &&
        source.parentId !== parentId &&
        (!parentId ||
          getFolderMoveDestinations(folders, source.id).some(
            (f) => f.id === parentId,
          ))
      );
    };
    return {
      onDragStart(event: DragEvent<HTMLElement>) {
        const source = targetAt(event);
        if (!source?.dataset.folderId || !source.draggable) return;
        draggedFolderId = source.dataset.folderId;
        event.dataTransfer.setData(FOLDER_MIME, draggedFolderId);
        event.dataTransfer.effectAllowed = "move";
        event.stopPropagation();
      },
      onDragOver(event: DragEvent<HTMLElement>) {
        if (!draggedFolderId) return;
        clearHighlight();
        const target = targetAt(event);
        if (canDrop(target)) {
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
          target!.classList.add("folder-drag-over");
        }
      },
      onDragLeave(event: DragEvent<HTMLElement>) {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null))
          clearHighlight();
      },
      onDrop(event: DragEvent<HTMLElement>) {
        const target = targetAt(event);
        if (!canDrop(target)) return;
        event.preventDefault();
        event.stopPropagation();
        const sourceId = draggedFolderId!;
        draggedFolderId = null;
        clearHighlight();
        void useLibraryFoldersStore
          .getState()
          .moveFolder(sourceId, target!.dataset.folderId ?? null);
      },
      onDragEnd() {
        draggedFolderId = null;
        clearHighlight();
      },
    };
  }, []);
}
