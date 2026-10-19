import Link from "next/link";
import { ArrowUpRightIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import { Badge } from "@/components/ui";
import { trackAttrs } from "@/lib/analytics";
import {
  craftAppName,
  craftAppPath,
  craftAppRelease,
  craftShotUrl,
  type CraftApp,
} from "@/lib/crafting-apps";
import { AppIcon, AppTab, AppWordmark, appThemeClass } from "./app-wordmark";

// Lineup cell: color tab + category, the hero screenshot, the app icon
// straddling the screenshot's bottom edge, wordmark, pitch, and (unless
// compact) status/platform chips. The whole cell is the link; a hairline in
// the app's color draws across the top on hover.
export default async function AppCard({
  app,
  index,
  compact = false,
}: {
  app: CraftApp;
  index: string;
  compact?: boolean;
}) {
  const [hero] = app.shots;
  const release = await craftAppRelease(app);
  return (
    <Link
      href={craftAppPath(app)}
      data-reveal
      {...trackAttrs("app_select", { app_name: app.slug })}
      className={twMerge(
        "group relative flex h-full flex-col bg-bg",
        appThemeClass(app),
      )}
    >
      <span
        aria-hidden
        className="absolute inset-x-0 top-0 z-10 h-0.5 origin-left scale-x-0 bg-(--app) transition-transform duration-300 group-hover:scale-x-100"
      />
      <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-3">
        <AppTab>{index}</AppTab>
        <p className="hud-label truncate text-faint">{app.category}</p>
      </div>
      <div className="relative aspect-[16/10] overflow-hidden border-b border-line bg-bg-sunken">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src={craftShotUrl(app, hero)}
          alt={hero.alt}
          loading="lazy"
          decoding="async"
          className="h-full w-full object-cover transition-transform duration-500 ease-out group-hover:scale-[1.03]"
        />
      </div>
      <div className={twMerge("flex flex-1 flex-col", compact ? "px-6 pb-6" : "px-6 pb-6 md:px-8 md:pb-8")}>
        <AppIcon
          app={app}
          className={twMerge(
            "relative transition-transform duration-300 ease-out group-hover:-translate-y-1",
            compact ? "-mt-7 h-14 w-14" : "-mt-9 h-18 w-18",
          )}
        />
        <h3 className="mt-4 leading-none">
          <AppWordmark app={app} className={compact ? "text-2xl" : "text-3xl"} />
        </h3>
        <p className="mt-3 leading-relaxed text-muted">{app.pitch}</p>
        {!compact && (
          <div className="mt-6 flex flex-wrap gap-1.5">
            <Badge label={app.status} className="text-(--app-ink)" />
            {release && (
              <Badge label="Installers ready" className="text-(--app-ink)" />
            )}
            {app.platforms.map((platform) => (
              <Badge key={platform} label={platform} />
            ))}
          </div>
        )}
        <span className="hud-label mt-auto flex items-center gap-1.5 pt-6 text-muted group-hover:text-ink">
          Explore {craftAppName(app)}
          <ArrowUpRightIcon
            aria-hidden
            className="h-3.5 w-3.5 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5"
          />
        </span>
      </div>
    </Link>
  );
}
