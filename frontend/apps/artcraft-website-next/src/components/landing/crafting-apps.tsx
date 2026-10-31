import Link from "next/link";
import { ArrowUpRightIcon } from "lucide-react";
import { AppIcon, appThemeClass } from "@/components/apps/app-wordmark";
import { Accent } from "@/components/page/page-header";
import { trackAttrs } from "@/lib/analytics";
import {
  CRAFTING_APPS,
  CRAFTING_APPS_COUNT_WORD,
  craftAppName,
  craftAppPath,
} from "@/lib/crafting-apps";
import { SectionShell, SectionEyebrow } from "./section-shell";

// Phones: icon beside the name, so seven rows stay short. From sm up: icon
// stacked over the name, seven across.
const CELL_CLASSES =
  "group relative flex items-center gap-3 bg-bg px-4 py-4 sm:flex-col sm:px-3 sm:py-6 sm:text-center md:py-8";
const ICON_CLASSES = "h-10 w-10 shrink-0 sm:h-14 sm:w-14 md:h-16 md:w-16";

// The Crafting Apps family on the landing page: one hairline cell per app
// (icon and name) plus a closing cell into the /apps hub, so the grid
// stays even (14 cells: two columns on phones, seven across from sm up).
export default function CraftingApps({ index }: { index: string }) {
  return (
    <SectionShell id="apps">
      <SectionEyebrow
        index={index}
        label="Crafting Apps"
        annotation="Open source · Pure Rust"
      />

      <div className="flex flex-col gap-4 px-6 py-14 md:flex-row md:items-end md:justify-between md:px-10 md:py-20">
        <div>
          <h2
            data-reveal
            className="max-w-3xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl md:text-6xl"
          >
            {CRAFTING_APPS_COUNT_WORD} apps. One <Accent>craft</Accent>.
          </h2>
          <p data-reveal className="mt-5 max-w-xl text-lg leading-relaxed text-muted">
            Native, open-source desktop apps from the ArtCraft team, built in
            Rust and free to use.
          </p>
        </div>
        <Link
          href="/apps"
          data-reveal
          className="hud-label flex shrink-0 items-center gap-1.5 text-muted hover:text-ink"
        >
          All Crafting Apps
          <ArrowUpRightIcon aria-hidden className="h-3.5 w-3.5" />
        </Link>
      </div>

      <nav
        aria-label="Crafting Apps"
        data-reveal-group
        className="grid grid-cols-2 gap-px border-t border-line bg-line sm:grid-cols-7"
      >
        {CRAFTING_APPS.map((app) => (
          <Link
            key={app.slug}
            href={craftAppPath(app)}
            data-reveal
            className={`${CELL_CLASSES} ${appThemeClass(app)}`}
            {...trackAttrs("app_select", { app_name: app.slug })}
          >
            <span
              aria-hidden
              className="absolute inset-x-0 top-0 h-0.5 origin-left scale-x-0 bg-(--app) transition-transform duration-300 group-hover:scale-x-100"
            />
            <AppIcon
              app={app}
              className={`${ICON_CLASSES} transition-transform duration-300 ease-out group-hover:-translate-y-1`}
            />
            <span className="text-sm font-medium text-ink group-hover:text-(--app-ink)">
              {craftAppName(app)}
            </span>
          </Link>
        ))}
        <Link href="/apps" data-reveal className={CELL_CLASSES}>
          <span className={`${ICON_CLASSES} flex items-center justify-center border border-line-strong text-ink transition-colors group-hover:bg-invert-bg group-hover:text-invert-fg`}>
            <ArrowUpRightIcon aria-hidden className="h-5 w-5 sm:h-6 sm:w-6" />
          </span>
          <span className="text-sm font-medium text-ink">All apps</span>
        </Link>
      </nav>
    </SectionShell>
  );
}
