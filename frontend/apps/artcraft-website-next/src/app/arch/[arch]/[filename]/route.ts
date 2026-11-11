import {
  handlePackageDownloadRedirect,
  isPackageSignaturePublished,
  VALID_PACMAN_ARCHES,
} from "@/lib/linux-repos";
import { isRepoSigningConfigured } from "@/lib/repo-signing";

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

  // pacman asks for `<package>.pkg.tar.zst.sig` next to each package and verifies it against the
  // imported ArtCraft key. A deployment that publishes no key cannot verify anything, and its
  // configs stay on the trusted fallback, so it must answer 404 rather than hand pacman a
  // signature it has no key for. A release that publishes no signature gets the same answer.
  if (filename.endsWith(".sig")) {
    const configured = await isRepoSigningConfigured();
    if (!configured || !(await isPackageSignaturePublished(filename))) {
      return new Response("Signature not found", { status: 404 });
    }
  }

  return handlePackageDownloadRedirect([filename]);
}
