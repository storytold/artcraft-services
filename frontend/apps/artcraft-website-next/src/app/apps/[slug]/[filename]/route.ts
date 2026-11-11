import { getCraftApp, craftAppName } from "@/lib/crafting-apps";
import { getFlatpakSignatureDescriptor } from "@/lib/flatpak-signatures";

type RouteProps = {
  params: Promise<{ slug: string; filename: string }>;
};

export async function GET(request: Request, props: RouteProps) {
  const { slug, filename } = await props.params;
  if (filename !== `${slug}-oci.flatpakref`) {
    return new Response("Not found", { status: 404 });
  }

  const app = getCraftApp(slug);

  if (!app) {
    return new Response("Not found", { status: 404 });
  }

  const url = new URL(request.url);
  const baseUrl = `${url.protocol}//${url.host}`;
  const title = craftAppName(app);
  const appId = `ai.storyteller.${slug}`;
  // The ref carries the same key and lookaside as the remote descriptor, so installing a single
  // .flatpakref verifies signatures instead of trusting the image.
  const signatureEntries = await getFlatpakSignatureDescriptor(baseUrl);

  const ini = `[Flatpak Ref]
Name=${appId}
Branch=stable
Title=${title}
Url=oci+${baseUrl}
RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo
IsRuntime=false
${signatureEntries.map((entry) => `${entry}\n`).join("")}`;

  return new Response(ini, {
    headers: {
      "Content-Type": "application/x-flatpak-ref; charset=utf-8",
      "Cache-Control": "public, max-age=86400",
      "Content-Disposition": `attachment; filename="${slug}-oci.flatpakref"`,
    },
  });
}
