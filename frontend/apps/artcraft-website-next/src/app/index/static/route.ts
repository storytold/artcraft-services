import { getFlatpakOciIndex } from "@/lib/flatpak-oci";

export const dynamic = "force-static";

export function GET(request: Request) {
  const url = new URL(request.url);
  const catalog = getFlatpakOciIndex(url.searchParams);

  return Response.json(catalog, {
    headers: {
      "Cache-Control": "public, max-age=300, s-maxage=300, stale-while-revalidate=86400",
      "Content-Type": "application/json",
    },
  });
}
