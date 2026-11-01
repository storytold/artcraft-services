import { handlePackageDownloadRedirect, VALID_PACMAN_ARCHES } from "@/lib/linux-repos";

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
    return new Response("Signature not found", { status: 404 });
  }

  return handlePackageDownloadRedirect([filename]);
}
