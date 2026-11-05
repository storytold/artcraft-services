import { getRpmRepomdXml, VALID_RPM_ARCHES } from "@/lib/linux-repos";
import { signDetachedArmored } from "@/lib/repo-signing";

type RouteProps = {
  params: Promise<{
    arch: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { arch } = await props.params;
  if (!VALID_RPM_ARCHES.has(arch)) {
    return new Response(`Architecture "${arch}" not supported`, { status: 404 });
  }

  const signature = await signDetachedArmored(await getRpmRepomdXml(arch));
  if (!signature) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  return new Response(signature, {
    headers: {
      "Content-Type": "application/pgp-signature",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
