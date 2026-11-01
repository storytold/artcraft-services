import { flatpakOciIndexResponse } from "@/lib/flatpak-oci-route";

export function GET(request: Request) {
  return flatpakOciIndexResponse(request, "public, max-age=300, s-maxage=300, stale-while-revalidate=86400");
}
