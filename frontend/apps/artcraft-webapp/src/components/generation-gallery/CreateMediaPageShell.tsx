import { type ReactNode, useState } from "react";
import { LoaderCircleIcon } from "lucide-react";
import { type PopoverItem } from "@storyteller/ui-popover";
import { TabSelector } from "@storyteller/ui-tab-selector";
import Seo from "../../components/seo";
import { useIsMobile } from "../ui/use-mobile";
import {
  GalleryAutoplayToggle,
  GallerySelectToggle,
  GalleryViewToggle,
} from "./GalleryViewToggle";
import {
  MobileCreateTabsProvider,
  type MobileCreateTab,
} from "./mobile-create-tabs";

interface CreateMediaPageShellProps {
  // SEO
  title: string;
  description: string;
  // Auth state - `authChecked` gates the initial spinner so we don't flash
  // logged-out chrome while the session resolves. Pages stay viewable for
  // logged-out users; the signup CTA modal is triggered at generate time.
  authChecked: boolean;
  // Content
  hasContent: boolean;
  // Video page only: playing/still toggle for video preview thumbnails,
  // rendered next to the grid/list toggle on the mobile History tab.
  showAutoplayToggle?: boolean;
  // Multi-select + batch download toggle, rendered in the same mobile
  // History-tab cluster (the desktop toggle lives in the TopBar).
  showSelectToggle?: boolean;
  emptyStateTitle: string;
  emptyStateSubtitle: string;
  // Optional CTA rendered under the empty-state subtitle (e.g. a signup button
  // for logged-out visitors). Omitted for logged-in users.
  emptyStateCta?: ReactNode;
  bottomOffset: number;
  // Model selector
  modelItems: PopoverItem[];
  onModelChange: (item: PopoverItem) => void;
  // Children slots
  gridContent: ReactNode;
  promptBox: ReactNode;
  // Mobile-only form rendered on the "Generate" tab. When provided and the
  // viewport is mobile, the shell swaps the floating prompt box for a tabbed
  // Generate / History layout.
  promptForm?: ReactNode;
  modals: ReactNode;
}

const MOBILE_TABS = [
  { id: "generate", label: "Generate" },
  { id: "history", label: "History" },
];

export function CreateMediaPageShell({
  title,
  description,
  authChecked,
  hasContent,
  showAutoplayToggle = false,
  showSelectToggle = false,
  emptyStateTitle,
  emptyStateSubtitle,
  emptyStateCta,
  bottomOffset,
  gridContent,
  promptBox,
  promptForm,
  modals,
}: CreateMediaPageShellProps) {
  const viewportIsMobile = useIsMobile();
  const [mobileTab, setMobileTab] = useState<MobileCreateTab>("generate");

  if (!authChecked) {
    return (
      <div className="flex h-full items-center justify-center bg-ui-background">
        <LoaderCircleIcon

          className="animate-spin text-4xl text-white/60" />
      </div>
    );
  }

  // Mobile: split into Generate (form) / History (gallery) tabs so the prompt
  // box no longer overlaps results. Desktop keeps the floating prompt box.
  if (viewportIsMobile && promptForm) {
    return (
      <MobileCreateTabsProvider
        value={{
          tab: mobileTab,
          setTab: setMobileTab,
          goToHistory: () => setMobileTab("history"),
        }}
      >
        <div className="flex h-full w-full flex-col bg-ui-background text-white">
          <Seo title={title} description={description} />

          {/* Flex (not absolute) keeps the toggle cluster from overlapping
              the tabs - the tabs stay centered via the equal flex-1 sides and
              just shift left if the cluster outgrows its half. */}
          <div className="flex flex-wrap items-center gap-y-2 border-b border-ui-panel-border bg-ui-panel px-3 py-2">
            <div className="flex-1" />
            <TabSelector
              tabs={MOBILE_TABS}
              activeTab={mobileTab}
              onTabChange={(id) => setMobileTab(id as MobileCreateTab)}
              className="w-auto shrink-0"
            />
            <div className="flex flex-1 items-center justify-end gap-1.5">
              {mobileTab === "history" && hasContent && (
                <>
                  {showSelectToggle && <GallerySelectToggle />}
                  {showAutoplayToggle && <GalleryAutoplayToggle />}
                  <GalleryViewToggle />
                </>
              )}
            </div>
          </div>

          <div className="min-h-0 flex-1 overflow-hidden">
            {mobileTab === "generate" ? (
              promptForm
            ) : hasContent ? (
              <div className="h-full w-full overflow-y-auto pt-0.5">
                <div className="px-3">{gridContent}</div>
              </div>
            ) : (
              <div className="relative isolate flex h-full flex-col items-center justify-center px-6 text-center">
                <div aria-hidden="true" className="create-empty-backdrop" />
                <span className="relative z-[1] font-display text-lg text-ui-ink">
                  {emptyStateTitle}
                </span>
                <span className="relative z-[1] pt-1 text-sm text-white/60">
                  Your generations will appear here.
                </span>
              </div>
            )}
          </div>
        </div>

        {modals}
      </MobileCreateTabsProvider>
    );
  }

  return (
    <div className="relative isolate flex h-full w-full bg-ui-background text-white">
      <Seo title={title} description={description} />

      {/* Static landing-style rails stay inside the workspace and never
          intercept input or cover the gallery once results arrive. */}
      {!hasContent && (
        <div aria-hidden="true" className="create-empty-backdrop" />
      )}

      <div className="relative z-[1] h-full w-full">
        <div className="flex h-full w-full flex-col">
          {!hasContent && (
            <div className="flex flex-1 items-center justify-center">
              <div className="animate-fade-in-up relative z-20 mb-32 flex max-w-3xl flex-col items-center justify-center px-6 text-center">
                <h1 className="font-display text-3xl leading-tight text-ui-ink md:text-5xl">
                  {emptyStateTitle}
                </h1>
                <span className="max-w-xl pt-3 text-sm leading-relaxed text-base-fg/70 md:text-base">
                  {emptyStateSubtitle}
                </span>
                {emptyStateCta && <div className="pt-6">{emptyStateCta}</div>}
              </div>
            </div>
          )}

          {hasContent && (
            <div
              className="h-full w-full overflow-y-auto pt-0.5"
              style={{ paddingBottom: bottomOffset }}
            >
              <div className="px-3">{gridContent}</div>
            </div>
          )}

          {/* Bottom fade behind the floating prompt box so it stays legible
              over the feed. Sits under the prompt box (z-30). */}
          {hasContent && (
            <div
              aria-hidden
              className="pointer-events-none fixed bottom-0 right-0 z-20 h-48 bg-gradient-to-t from-ui-background to-transparent"
              style={{ left: "var(--ac-sidebar-offset, 0px)" }}
            />
          )}

          {promptBox}
        </div>
      </div>

      {modals}
    </div>
  );
}
