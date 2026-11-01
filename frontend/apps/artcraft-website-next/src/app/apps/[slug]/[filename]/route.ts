import { getCraftApp, craftAppName } from "@/lib/crafting-apps";

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

  const ini = `[Flatpak Ref]
Name=${appId}
Branch=stable
Title=${title}
Url=oci+${baseUrl}
RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo
IsRuntime=false
`;

  return new Response(ini, {
    headers: {
      "Content-Type": "application/x-flatpak-ref; charset=utf-8",
      "Cache-Control": "public, max-age=86400",
      "Content-Disposition": `attachment; filename="${slug}-oci.flatpakref"`,
    },
  });
}
