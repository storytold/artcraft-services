import { generateAptRelease, VALID_APT_DISTRIBUTIONS } from "@/lib/linux-repos";

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

  const content = generateAptRelease();

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=60, s-maxage=60",
    },
  });
}
