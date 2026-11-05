import { generateAptRelease, VALID_APT_DISTRIBUTIONS } from "@/lib/linux-repos";
import { isRepoSigningConfigured, signCleartext } from "@/lib/repo-signing";

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

  // Check before generating: without a usable key this route is a 404, not a cold-cache fetch.
  if (!(await isRepoSigningConfigured())) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  const inRelease = await signCleartext(await generateAptRelease());
  if (!inRelease) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  return new Response(inRelease, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
