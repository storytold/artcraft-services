import { flatpakOciIndexResponse } from "@/lib/flatpak-oci-route";

export function GET(request: Request) {
  return flatpakOciIndexResponse(request, "no-cache");
}
