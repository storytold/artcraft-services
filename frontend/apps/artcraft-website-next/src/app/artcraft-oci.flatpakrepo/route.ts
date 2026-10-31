import { getRequestBaseUrl } from "@/lib/linux-repos";

export function GET(request: Request) {
  const baseUrl = getRequestBaseUrl(request);

  const ini = `[Flatpak Repo]
Title=ArtCraft OCI
Url=oci+${baseUrl}
Homepage=https://getartcraft.com/
Comment=Official ArtCraft OCI Flatpak repository
Description=Open-source creative apps for Linux distributed via GitHub Container Registry
GPGVerify=false
`;

  return new Response(ini, {
    headers: {
      "Content-Type": "application/x-flatpak-repo; charset=utf-8",
      "Cache-Control": "public, max-age=86400",
      "Content-Disposition": 'attachment; filename="artcraft-oci.flatpakrepo"',
    },
  });
}
