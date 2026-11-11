import { LayoutGridIcon } from "lucide-react";
import AppIconRow from "@/components/apps/app-icon-row";
import { Button } from "@/components/ui";
import { CRAFT_LAUNCHER_NAME } from "@/lib/craft-launcher";
import { CRAFTING_APPS_COUNT_WORD } from "@/lib/crafting-apps";

// Signpost on the ArtCraft Studio download page for visitors who came for
// the Crafting Apps, which are a separate download. A server component, so
// the app catalog stays out of the download page's client bundle.
export default function CraftingAppsCallout() {
  return (
    <aside
      data-reveal
      aria-label="Crafting Apps"
      className="mt-12 border border-line bg-bg-raised p-6 xl:mt-0 xl:w-96 xl:shrink-0"
    >
      <AppIconRow />
      <h2 className="mt-5 text-balance font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
        Looking for the Crafting Apps?
      </h2>
      <p className="mt-2 leading-relaxed text-muted">
        PhotoCraft, VectorCraft and the rest of the{" "}
        {CRAFTING_APPS_COUNT_WORD.toLowerCase()} open-source apps are a separate
        download, installed and kept up to date by {CRAFT_LAUNCHER_NAME}.
      </p>
      <Button href="/apps" variant="secondary" className="mt-6">
        <LayoutGridIcon aria-hidden className="h-4 w-4" />
        Explore Crafting Apps
      </Button>
    </aside>
  );
}
