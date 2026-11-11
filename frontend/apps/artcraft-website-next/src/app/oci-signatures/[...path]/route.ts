import { getOciImageSignature, parseSignatureLookasidePath } from "@/lib/flatpak-signatures";

type RouteProps = {
  params: Promise<{
    path: string[];
  }>;
};

/**
 * containers/image signature lookaside consumed by Flatpak for OCI pulls.
 * @see https://github.com/containers/image/blob/main/docs/signature-protocols.md
 */
export async function GET(_request: Request, props: RouteProps) {
  const { path } = await props.params;
  const request = parseSignatureLookasidePath(path ?? []);
  if (!request) {
    return new Response("Not found", { status: 404 });
  }

  // Exactly one signature per image; a 404 here is how Flatpak stops asking for more.
  if (request.index !== 1) {
    return new Response("Not found", { status: 404 });
  }

  const signature = await getOciImageSignature(request.repository, request.digest);
  if (!signature) {
    return new Response("Signature not found", { status: 404 });
  }

  return new Response(new Uint8Array(signature), {
    headers: {
      "Content-Type": "application/pgp-signature",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
