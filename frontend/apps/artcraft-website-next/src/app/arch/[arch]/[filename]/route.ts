import { handlePackageDownloadRedirect, resolvePackageSignatureUrl, VALID_PACMAN_ARCHES } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    arch: string;
    filename: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { arch, filename } = await props.params;
  if (!VALID_PACMAN_ARCHES.has(arch)) {
    return new Response(`Architecture "${arch}" not supported`, { status: 404 });
  }

  if (filename.endsWith(".sig")) {
    const signatureUrl = resolvePackageSignatureUrl(filename);
    return signatureUrl ? Response.redirect(signatureUrl, 302) : new Response("Signature not found", { status: 404 });
  }

  return handlePackageDownloadRedirect([filename]);
}
