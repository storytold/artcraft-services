import { getRequestBaseUrl } from "@/lib/linux-repos";
import { flatpakRepoDescriptor } from "@/lib/flatpak-oci";

export async function GET(request: Request) {
  const ini = await flatpakRepoDescriptor(getRequestBaseUrl(request));

  return new Response(ini, {
    headers: {
      "Content-Type": "application/x-flatpak-repo; charset=utf-8",
      "Cache-Control": "public, max-age=86400",
      "Content-Disposition": 'attachment; filename="artcraft-oci.flatpakrepo"',
    },
  });
}
