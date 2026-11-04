import { resolveOciSignatureUrl } from "@/lib/flatpak-oci";

type RouteProps = {
  params: Promise<{ path: string[] }>;
};

/** Flatpak's signature lookaside for the ArtCraft OCI remote (`SignatureLookaside=…/signatures`). */
export async function GET(_request: Request, props: RouteProps) {
  const { path } = await props.params;
  try {
    const signatureUrl = await resolveOciSignatureUrl(path.map(decodeURIComponent).join("/"));
    return signatureUrl ? Response.redirect(signatureUrl, 302) : new Response("Signature not found", { status: 404 });
  } catch {
    return new Response("Signature lookup is unavailable", { status: 503, headers: { "Cache-Control": "no-store" } });
  }
}
