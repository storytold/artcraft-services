import { CRAFTING_APPS } from "@/lib/crafting-apps";
import { AppIcon } from "./app-wordmark";

// A short, decorative preview of the family: the first few app icons and a
// "+N" chip for the rest.
export default function AppIconRow({ count = 6 }: { count?: number }) {
  const more = CRAFTING_APPS.length - count;
  return (
    <div aria-hidden className="flex items-center gap-1.5">
      {CRAFTING_APPS.slice(0, count).map((app) => (
        <AppIcon key={app.slug} app={app} className="h-9 w-9" />
      ))}
      {more > 0 && (
        <span className="hud-label flex h-9 w-9 items-center justify-center border border-line text-faint">
          +{more}
        </span>
      )}
    </div>
  );
}
