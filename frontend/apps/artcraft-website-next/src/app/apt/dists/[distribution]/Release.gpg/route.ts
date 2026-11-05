import { generateAptRelease, VALID_APT_DISTRIBUTIONS } from "@/lib/linux-repos";
import { signDetachedArmored } from "@/lib/repo-signing";

type RouteProps = {
  params: Promise<{
    distribution: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { distribution } = await props.params;

  if (!VALID_APT_DISTRIBUTIONS.has(distribution)) {
    return new Response("Not found", { status: 404 });
  }

  const signature = await signDetachedArmored(await generateAptRelease());
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
