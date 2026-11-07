import { clearGalleryCache } from "@storyteller/ui-gallery-modal";
import { clearAnimationLibraryAccountState } from "@storyteller/ui-pagescene";
import { useVFXStore } from "@storyteller/ui-vfx";
import { useCreateVideoStore } from "../pages/create-video/create-video-store";
import { useLibraryFoldersStore } from "../pages/library/library-folders-store";
import { useLibraryTagsStore } from "../pages/library/library-tags-store";
import { useSceneCacheStore } from "../pages/pagescene/scene-cache-store";
import { onAccountChange } from "./session";

// Every cache that holds one account's data, dropped when the signed-in
// identity changes (see reconcileAccountOwner). Imported once from main.tsx.
onAccountChange(() => {
  useCreateVideoStore.getState().resetAccountState();
  useLibraryFoldersStore.getState().resetAccountState();
  useLibraryTagsStore.getState().resetAccountState();
  useSceneCacheStore.getState().resetAccountState();
  useVFXStore.getState().reset();
  clearGalleryCache();
  clearAnimationLibraryAccountState();
});
