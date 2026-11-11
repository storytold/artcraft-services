import type { ReactNode } from "react";
import { twMerge } from "tailwind-merge";
import { craftAppIconUrl, type CraftApp } from "@/lib/crafting-apps";

// The app's engraved-animal icon: a rounded tile with transparent corners,
// so the drop shadow follows the tile shape. Decorative — it always sits
// beside the app's name.
export function AppIcon({
  app,
  priority = false,
  className,
}: {
  app: CraftApp;
  /** Above the fold: load eagerly at high priority. */
  priority?: boolean;
  className?: string;
}) {
  return (
    // eslint-disable-next-line @next/next/no-img-element
    <img
      src={craftAppIconUrl(app)}
      alt=""
      width={256}
      height={256}
      loading={priority ? "eager" : "lazy"}
      fetchPriority={priority ? "high" : undefined}
      decoding="async"
      className={twMerge(
        "h-14 w-14 shrink-0 drop-shadow-[0_6px_14px_rgba(0,0,0,0.28)]",
        className,
      )}
    />
  );
}

// Scopes the app's color identity (`--app`, `--app-ink` in globals.css).
export function appThemeClass(app: CraftApp): string {
  return `app-${app.slug}`;
}

// Text wordmark: the display face throughout, "Craft" picked out in the
// app's ink color. Needs an appThemeClass ancestor (or pass `themed`).
export function AppWordmark({
  app,
  themed = false,
  className,
}: {
  app: CraftApp;
  themed?: boolean;
  className?: string;
}) {
  return (
    <span
      className={twMerge(
        "font-display font-medium tracking-[-0.03em] text-ink-strong",
        themed && appThemeClass(app),
        className,
      )}
    >
      {app.prefix}
      <ColorAccent>Craft</ColorAccent>{app.nameSuffix}
    </span>
  );
}

// The Crafting Apps pages' contrast word: same face as the headline, set in
// the app's ink color (the site accent outside an app scope) instead of the
// serif italic used elsewhere on the site.
export function ColorAccent({ children }: { children: ReactNode }) {
  return (
    <span className="text-(--app-ink,var(--accent-ink))">{children}</span>
  );
}

// Saturated index tab in the app's color (black text passes AA on every app).
export function AppTab({ children }: { children: ReactNode }) {
  return (
    <span className="hud-label bg-(--app) px-2 py-1 font-bold text-black">
      {children}
    </span>
  );
}
