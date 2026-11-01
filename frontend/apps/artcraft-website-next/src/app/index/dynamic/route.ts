import { getFlatpakOciIndex } from "@/lib/flatpak-oci";

export function GET(request: Request) {
  const url = new URL(request.url);
  const catalog = getFlatpakOciIndex(url.searchParams);

  return Response.json(catalog, {
    headers: {
      "Cache-Control": "no-cache",
      "Content-Type": "application/json",
    },
  });
}
