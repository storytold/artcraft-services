import { generatePacmanDb, VALID_PACMAN_ARCHES } from "@/lib/linux-repos";
import { isRepoSigningConfigured, signDetachedBinary } from "@/lib/repo-signing";

type RouteProps = {
  params: Promise<{
    arch: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { arch } = await props.params;
  if (!VALID_PACMAN_ARCHES.has(arch)) {
    return new Response(`Architecture "${arch}" not supported`, { status: 404 });
  }

  if (!(await isRepoSigningConfigured())) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  const dbBuffer = await generatePacmanDb(arch);
  const signature = await signDetachedBinary(new Uint8Array(dbBuffer));
  if (!signature) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  return new Response(new Uint8Array(signature), {
    headers: {
      "Content-Type": "application/pgp-signature",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
