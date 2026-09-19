import { useMemo, useRef, useState } from "react";
import { Modal } from "@storyteller/ui-modal";
import { Button } from "@storyteller/ui-button";
import { ChevronDownIcon } from "lucide-react";
import { getFolderMoveDestinations, getFolderPath } from "./folder-move";
import { useLibraryFoldersStore, type UiFolder } from "./library-folders-store";

export function MoveFolderDialog({
  folder,
  onClose,
}: {
  folder: UiFolder;
  onClose: () => void;
}) {
  const folders = useLibraryFoldersStore((s) => s.folders);
  const moveFolder = useLibraryFoldersStore((s) => s.moveFolder);
  const [destination, setDestination] = useState(folder.parentId ?? "");
  const [moving, setMoving] = useState(false);
  const inFlight = useRef(false);
  const destinations = useMemo(() => {
    const byId = new Map(folders.map((f) => [f.id, f]));
    return getFolderMoveDestinations(folders, folder.id)
      .map((f) => ({ id: f.id, path: getFolderPath(f, byId) }))
      .sort((a, b) => a.path.localeCompare(b.path));
  }, [folders, folder.id]);
  const validDestination =
    destination === "" || destinations.some((f) => f.id === destination);

  const submit = async () => {
    if (
      inFlight.current ||
      !validDestination ||
      destination === (folder.parentId ?? "")
    )
      return;
    inFlight.current = true;
    setMoving(true);
    try {
      if (await moveFolder(folder.id, destination || null)) onClose();
    } finally {
      inFlight.current = false;
      setMoving(false);
    }
  };

  return (
    <Modal
      isOpen
      onClose={() => {
        if (!inFlight.current) onClose();
      }}
      title="Move folder"
      accessibleTitle="Move folder"
      className="w-96 max-w-[90vw] [&_h2]:font-sans [&_h2]:text-sm [&_h2]:font-semibold [&_h2]:tracking-normal [&_h2]:[font-stretch:normal]"
    >
      <div className="space-y-3">
        <p className="text-sm text-base-fg/70">
          Move “{folder.name}” with all its items and subfolders.
        </p>
        <label className="block space-y-1 text-sm">
          <span>Destination</span>
          <div className="relative">
            <select
              value={destination}
              onChange={(e) => setDestination(e.target.value)}
              disabled={moving}
              autoFocus
              className="w-full appearance-none rounded-[3px] border border-ui-panel-border bg-ui-panel py-2 pl-3 pr-10 text-sm text-base-fg outline-none focus:ring-1 focus:ring-primary/50"
            >
              <option value="">Folders (top level)</option>
              {destinations.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.path}
                </option>
              ))}
            </select>
            <ChevronDownIcon
              aria-hidden="true"
              className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-base-fg/70"
            />
          </div>
        </label>
        <div className="flex justify-end gap-2">
          <Button variant="action" onClick={onClose} disabled={moving}>
            Cancel
          </Button>
          <Button
            onClick={submit}
            disabled={
              moving ||
              !validDestination ||
              destination === (folder.parentId ?? "")
            }
          >
            {moving ? "Moving…" : "Move"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
