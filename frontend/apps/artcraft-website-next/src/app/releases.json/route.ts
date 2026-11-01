import {
  CRAFTING_APPS,
  craftAppName,
  craftAppRelease,
  craftAppRepo,
  craftReleaseDownloads,
  craftReleasePageUrl,
} from "@/lib/crafting-apps";
import { craftLauncherDownloads, craftLauncherReleasePageUrl } from "@/lib/craft-launcher";
import { CRAFT_LAUNCHER_RELEASE } from "@/lib/crafting-app-releases";

// Keep this public endpoint in sync with the catalog that drives website downloads.
export const dynamic = "force-static";

export function GET() {
  const apps = Object.fromEntries(
    CRAFTING_APPS.map((app) => {
      const release = craftAppRelease(app);

      return [
        app.slug,
        release
          ? {
              name: craftAppName(app),
              repository: craftAppRepo(app),
              version: release.version,
              tag: release.tag ?? `v${release.version}`,
              releaseUrl: craftReleasePageUrl(app, release),
              assets: craftReleaseDownloads(app, release),
            }
          : null,
      ];
    }),
  );

  return Response.json(
    {
      schemaVersion: 1,
      launcher: CRAFT_LAUNCHER_RELEASE
        ? {
            name: "ArtCraft Launcher",
            repository: "https://github.com/storytold/craft-launcher",
            version: CRAFT_LAUNCHER_RELEASE.version,
            tag: CRAFT_LAUNCHER_RELEASE.tag ?? `v${CRAFT_LAUNCHER_RELEASE.version}`,
            releaseUrl: craftLauncherReleasePageUrl(),
            assets: craftLauncherDownloads(),
          }
        : null,
      apps,
    },
    {
      headers: {
        "Cache-Control": "public, max-age=300, s-maxage=300, stale-while-revalidate=86400",
      },
    },
  );
}
