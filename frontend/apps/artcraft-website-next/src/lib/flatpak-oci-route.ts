import { getFlatpakOciIndex } from "./flatpak-oci";

export async function flatpakOciIndexResponse(request: Request, cacheControl: string): Promise<Response> {
  const url = new URL(request.url);
  try {
    const catalog = await getFlatpakOciIndex(url.searchParams);
    return Response.json(catalog, {
      headers: {
        "Cache-Control": cacheControl,
        "Content-Type": "application/json",
        "Netlify-Vary": "query",
      },
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Flatpak OCI index is unavailable";
    return Response.json({ error: message }, {
      status: 503,
      headers: { "Cache-Control": "no-store", "Netlify-Vary": "query" },
    });
  }
}
