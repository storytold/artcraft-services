import { getFlatpakOciIndex } from "@/lib/flatpak-oci";

export async function GET(request: Request) {
  const url = new URL(request.url);
  const catalog = await getFlatpakOciIndex(url.searchParams);

  return Response.json(catalog, {
    headers: {
      "Cache-Control": "no-cache",
      "Content-Type": "application/json",
    },
  });
}
