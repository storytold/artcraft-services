import { getRequestBaseUrl } from "@/lib/linux-repos";
import { getFlatpakSignatureDescriptor } from "@/lib/flatpak-signatures";

export async function GET(request: Request) {
  const baseUrl = getRequestBaseUrl(request);
  const signatureEntries = await getFlatpakSignatureDescriptor(baseUrl);

  const ini = `[Flatpak Repo]
Title=ArtCraft OCI
Url=oci+${baseUrl}
Homepage=https://getartcraft.com/
Comment=Official ArtCraft OCI Flatpak repository
Description=Open-source creative apps for Linux distributed via GitHub Container Registry
${signatureEntries.length > 0 ? `${signatureEntries.join("\n")}\n` : "GPGVerify=false\n"}`;

  return new Response(ini, {
    headers: {
      "Content-Type": "application/x-flatpak-repo; charset=utf-8",
      "Cache-Control": "public, max-age=86400",
      "Content-Disposition": 'attachment; filename="artcraft-oci.flatpakrepo"',
    },
  });
}
