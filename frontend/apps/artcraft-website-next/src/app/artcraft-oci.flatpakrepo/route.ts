export const dynamic = "force-static";

export function GET(request: Request) {
  const url = new URL(request.url);
  const baseUrl = `${url.protocol}//${url.host}`;

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
